//! The Parse Step: parse a bounded batch of the Inventory files whose facts are
//! missing or stale, and commit those facts with the job's transition, so a
//! restart resumes from the last commit and an unchanged file is never parsed
//! twice.

use super::super::inventory::Work;
use super::publish::{measure, publish, Batch};
use super::records::{self, FileState, Outcome, Status};
use super::worker::{Answer, Worker};
use super::*;
use crate::engine::transaction_admission::TransactionPermit;
use rrd_attunement::inventory::{Entry, EntryKind};
use rrd_attunement::parse::{parser_revision, ParseLimits};
use rrd_attunement::worker::{WorkerRequest, WORKER_PROTOCOL_VERSION};
use rrd_contract::SourceParsePolicy;
use std::collections::BTreeMap;
use std::time::Duration;

/// AST chunk bound in bytes. Fixed until the parse policy carries it.
const CHUNK_BYTES: u64 = 1_500;
/// Workers started in one Step; files after the last loss wait for the next Step.
const MAX_WORKERS: u32 = 3;

#[cfg(test)]
use super::worker::in_process as start_worker;
#[cfg(not(test))]
use super::worker::start as start_worker;

/// A file the policy binds to a grammar, with the state it should reach.
struct Wanted<'a> {
    entry: &'a Entry,
    bytes: u64,
    state: FileState,
}

fn wanted<'a>(entries: &'a [Entry], policy: &SourceParsePolicy, job_id: &str) -> Vec<Wanted<'a>> {
    let bound: BTreeMap<&str, &str> = policy
        .bindings
        .iter()
        .map(|binding| (binding.extension.as_str(), binding.language.as_str()))
        .collect();
    entries
        .iter()
        .filter_map(|entry| {
            let EntryKind::File {
                observation,
                content_sha256,
            } = &entry.kind
            else {
                return None;
            };
            let extension = entry.path.rsplit_once('.')?.1;
            let language = *bound.get(extension)?;
            let revision = parser_revision(language)?;
            Some(Wanted {
                entry,
                bytes: observation.bytes,
                state: FileState {
                    path: entry.path.clone(),
                    content_sha256: content_sha256.clone(),
                    language: language.into(),
                    parser_revision: revision.into(),
                    chunk_bytes: CHUNK_BYTES,
                    job_id: job_id.into(),
                    status: Status::Lost,
                    reason: String::new(),
                    facts_sha256: String::new(),
                    symbols: 0,
                    spans: 0,
                    chunks: 0,
                    nodes: 0,
                    error_nodes: 0,
                },
            })
        })
        .collect()
}

/// True when this job need not parse the file again: its content already has a
/// final answer, or the worker was lost on it in this job, which a later job retries.
fn settled(current: Option<&FileState>, wanted: &FileState) -> bool {
    current.is_some_and(|current| {
        current.content_sha256 == wanted.content_sha256
            && current.parser_revision == wanted.parser_revision
            && current.chunk_bytes == wanted.chunk_bytes
            && match current.status {
                Status::Parsed | Status::Rejected => true,
                Status::Lost => current.job_id == wanted.job_id,
            }
    })
}

fn encoded_len(value: &impl Serialize) -> Result<u64> {
    Ok(serde_json::to_vec(value).map_err(contract)?.len() as u64)
}

pub(in crate::engine::attunement) fn execute(
    engine: &RrdEngine,
    work: Work<'_>,
    mut permit: TransactionPermit<'_>,
) -> Result<AttunementControlResult> {
    let holder = super::super::worker(work.session)?;
    let invocation = engine.invocation_admission(
        work.context.request_id.as_str(),
        work.context.operation_id.as_str(),
    )?;
    let check = || {
        if let Some(invocation) = &invocation {
            invocation.check()?;
        }
        let at = elapsed_time(work.now, work.started)?;
        check_deadline(work.context, at)?;
        require_worker(&work.before, &holder, work.lease_generation, at)
    };
    check()?;
    let step = work
        .plan
        .phases
        .get(work.before.checkpoints.len())
        .filter(|step| step.phase == AttunementPhase::Parse)
        .ok_or_else(|| contract("selected phase is not Parse"))?;
    let policy = step
        .parse_policy
        .as_ref()
        .ok_or_else(|| contract("the Parse step has no parse policy"))?;
    let mut remaining_keys = engine.estate_configuration.query.max_storage_keys;
    let proposal = load_parse_inventory(engine, &work.before, &work.plan, &mut remaining_keys)?;
    let root = engine
        .installed_project_root
        .as_ref()
        .ok_or(ServiceError::ProjectBindingMismatch)?;
    let scope = ScopeId::new(format!("instance:{}", engine.instance)).map_err(contract)?;
    // The parsed-source tables and their indexes exist before any fact is
    // written, so every fact is indexed in the commit that writes it.
    let before_schema = engine.storage.runtime().read_stamp(&scope)?;
    if let Some(schema) = records::schema_mutation(engine, &scope, &before_schema)? {
        engine.storage.runtime().commit(&RuntimeCommit {
            scope: scope.clone(),
            at: elapsed_time(work.now, work.started)?,
            actor: format!("session:{}", work.session.as_str()),
            expected_cursor: before_schema.commit_cursor,
            mutations: vec![schema],
        })?;
    }
    records::ensure_indexes(
        engine,
        &scope,
        &rrd_query::IndexMutationContext {
            at: elapsed_time(work.now, work.started)?,
            actor: format!("session:{}", work.session.as_str()),
            request_id: work.context.request_id.as_str().into(),
            operation_id: work.context.operation_id.as_str().into(),
        },
    )?;
    let read = engine.storage.runtime().read_stamp(&scope)?;
    let states = records::load_states(engine, &read, &mut remaining_keys)?;
    let wanted = wanted(proposal.entries(), policy, work.before.id.as_str());
    let at = elapsed_time(work.now, work.started)?;
    let budget = policy.max_commit_bytes.min(
        engine
            .estate_configuration
            .storage
            .transaction_admission_bytes,
    );

    let mutations: Vec<RuntimeMutation> = records::schema_mutation(engine, &scope, &read)?
        .into_iter()
        .collect();
    let kept: std::collections::BTreeSet<&str> =
        wanted.iter().map(|file| file.state.path.as_str()).collect();
    let departed: Vec<&FileState> = states
        .values()
        .filter(|state| !kept.contains(state.path.as_str()))
        .collect();
    let base_bytes = encoded_len(&mutations)?;
    let mut used = base_bytes;
    let pending: Vec<&Wanted<'_>> = wanted
        .iter()
        .filter(|file| !settled(states.get(&file.state.path), &file.state))
        .collect();
    let mut remaining_input = policy.max_input_bytes;
    let mut worker: Option<Worker> = None;
    let mut started_workers = 0;
    let mut groups: Vec<Group> = Vec::new();
    let timeout = Duration::from_millis(policy.max_elapsed_ms);
    let limit = rrd_store::PHYSICAL_BATCH_PAYLOAD_BYTES;
    // The store's physical encoding of a batch is many times its JSON size,
    // so one measurement early in the batch sets how much JSON can be
    // gathered; the exact measurement before publishing is the authority.
    let mut cap = budget;
    // Files that left the project are retired first, one group each, within
    // the same bound as publication. What does not fit waits for the next
    // Step, so a large deletion spreads over Steps like a large project does.
    for state in &departed {
        check()?;
        let retirement = records::retire_file(state, at)?;
        let size = encoded_len(&retirement)?;
        if used.saturating_add(size) > cap && !groups.is_empty() {
            break;
        }
        used = used.saturating_add(size);
        groups.push(Group {
            mutations: retirement,
            answer: None,
        });
        // Removing a fact from its indexes costs far more than its JSON, so
        // retirements are measured from their first group.
        if groups.len() == 1 {
            let measured = measure_gathered(
                engine,
                &work,
                &check,
                &scope,
                &Gathered {
                    states: &states,
                    kept: &kept,
                    base: &mutations,
                    groups: &groups,
                },
            )?;
            cap = cap.min(fitting(used, measured, limit).max(base_bytes.saturating_add(1)));
        }
    }
    let retired = groups.len();
    // Parsing waits until every departed file is retired: a Step that cannot
    // publish what it parsed would only parse it again.
    let mut calibrated = false;
    for file in pending.iter().take(if retired == departed.len() {
        usize::MAX
    } else {
        0
    }) {
        if (groups.len() - retired) as u64 >= policy.max_files {
            break;
        }
        check()?;
        let previous = states.get(&file.state.path).cloned();
        let next = file.state.clone();
        let mut source = Vec::new();
        let outcome_mutations = if file.bytes > policy.max_file_bytes {
            records::file_mutations(
                previous.as_ref(),
                next.clone(),
                Outcome::Rejected("file_too_large".into()),
                &source,
                at,
            )?
        } else {
            if file.bytes > remaining_input {
                break;
            }
            crate::engine::installation::read_selected_source(
                root,
                &proposal,
                file.entry,
                policy.max_file_bytes,
                &mut remaining_input,
                &mut source,
                &check,
            )?;
            if worker.is_none() {
                if started_workers >= MAX_WORKERS {
                    break;
                }
                started_workers += 1;
                worker = Some(start_worker(policy.max_memory_bytes).map_err(|error| {
                    ServiceError::Storage(format!("parse worker did not start: {error}"))
                })?);
            }
            let request = WorkerRequest {
                format_version: WORKER_PROTOCOL_VERSION,
                source_id: file.entry.identity.clone(),
                content_sha256: next.content_sha256.clone(),
                language: next.language.clone(),
                source_bytes: source.len() as u64,
                limits: ParseLimits {
                    input_bytes: policy.max_file_bytes,
                    nodes: policy.max_nodes,
                    depth: policy.max_depth,
                    output_bytes: policy.max_output_bytes,
                    elapsed_micros: policy.max_elapsed_ms.saturating_mul(1_000),
                },
                chunk_bytes: CHUNK_BYTES,
            };
            let answer = worker.as_mut().expect("a worker was started above").parse(
                request,
                source.clone(),
                timeout,
            );
            let outcome = match &answer {
                Answer::Parsed(facts) => Outcome::Parsed(facts),
                Answer::Rejected(code) => Outcome::Rejected(code.clone()),
                Answer::Lost(reason) => {
                    worker = None;
                    Outcome::Lost(reason.clone())
                }
            };
            records::file_mutations(previous.as_ref(), next.clone(), outcome, &source, at)?
        };
        let size = encoded_len(&outcome_mutations)?;
        if used.saturating_add(size) > cap && !groups.is_empty() {
            break;
        }
        used = used.saturating_add(size);
        groups.push(Group {
            mutations: outcome_mutations,
            answer: Some(FileAnswer {
                previous,
                next,
                rejected: false,
            }),
        });
        if !calibrated && used >= (limit as u64) / 16 {
            calibrated = true;
            let measured = measure_gathered(
                engine,
                &work,
                &check,
                &scope,
                &Gathered {
                    states: &states,
                    kept: &kept,
                    base: &mutations,
                    groups: &groups,
                },
            )?;
            cap = cap.min(fitting(used, measured, limit).max(base_bytes.saturating_add(1)));
        }
    }
    drop(worker);
    // Parse is finished when every departed file is retired and every pending
    // file has its answer.
    let everything = departed.len() + pending.len();
    // Publish the longest prefix of the batch the store admits. A single file
    // whose facts cannot fit any batch is refused with the measured size and
    // is not retried until its content changes.
    let mut take = groups.len();
    let (batch_mutations, summary_sha256) = loop {
        let summary = summary(&states, &groups[..take], &kept)?;
        let batch_mutations = assemble(&mutations, &groups[..take]);
        let measured = measure(
            engine,
            &work,
            &check,
            &scope,
            Batch {
                mutations: batch_mutations.clone(),
                finished: take == everything,
                summary_sha256: &summary,
            },
        )?;
        if measured <= limit {
            break (batch_mutations, summary);
        }
        match take {
            0 => {
                return Err(ServiceError::StorageAdmission(format!(
                    "the Parse batch's base records measure {measured} bytes against the physical batch limit {limit}"
                )))
            }
            1 => match &mut groups[0].answer {
                // One file's retirement is not divisible here: its facts are
                // addressed by its state's counts. It is refused by name
                // rather than retried under another shape.
                None => {
                    return Err(ServiceError::StorageAdmission(format!(
                        "retiring {} measures {measured} bytes against the physical batch limit {limit}",
                        departed[0].path
                    )))
                }
                Some(answer) if !answer.rejected => {
                    let rejected = records::file_mutations(
                        answer.previous.as_ref(),
                        answer.next.clone(),
                        Outcome::Rejected(format!(
                            "facts_exceed_physical_batch:{measured}>{limit}"
                        )),
                        &[],
                        at,
                    )?;
                    answer.rejected = true;
                    groups[0].mutations = rejected;
                }
                Some(_) => take = 0,
            },
            _ => {
                let fit = (take as u128 * (limit as u128 * 9 / 10) / measured as u128) as usize;
                take = fit.clamp(1, take - 1);
            }
        }
    };
    let finished = take == everything;
    permit.charge(
        &batch_mutations,
        engine
            .estate_configuration
            .storage
            .transaction_admission_bytes,
    )?;
    publish(
        engine,
        &work,
        &check,
        &scope,
        Batch {
            mutations: batch_mutations,
            finished,
            summary_sha256: &summary_sha256,
        },
    )
}

/// One indivisible part of a batch: a departed file's retirement, or a
/// pending file's answer.
struct Group {
    mutations: Vec<RuntimeMutation>,
    /// `None` for a retirement.
    answer: Option<FileAnswer>,
}

/// The states either side of one file's answer.
struct FileAnswer {
    previous: Option<FileState>,
    next: FileState,
    rejected: bool,
}

/// What a Step has gathered so far, for one physical measurement.
struct Gathered<'a> {
    states: &'a BTreeMap<String, FileState>,
    kept: &'a std::collections::BTreeSet<&'a str>,
    base: &'a [RuntimeMutation],
    groups: &'a [Group],
}

/// Physical bytes the store would write for everything gathered so far.
fn measure_gathered(
    engine: &RrdEngine,
    work: &Work<'_>,
    check: &dyn Fn() -> Result<()>,
    scope: &ScopeId,
    gathered: &Gathered<'_>,
) -> Result<u64> {
    let summary = summary(gathered.states, gathered.groups, gathered.kept)?;
    Ok(measure(
        engine,
        work,
        check,
        scope,
        Batch {
            mutations: assemble(gathered.base, gathered.groups),
            finished: false,
            summary_sha256: &summary,
        },
    )? as u64)
}

/// JSON bytes a batch may gather at the measured physical ratio: nine tenths
/// of the physical ceiling.
fn fitting(used: u64, measured: u64, limit: usize) -> u64 {
    (u128::from(used) * (limit as u128 * 9 / 10) / u128::from(measured.max(1))) as u64
}

fn assemble(base: &[RuntimeMutation], groups: &[Group]) -> Vec<RuntimeMutation> {
    base.iter()
        .cloned()
        .chain(
            groups
                .iter()
                .flat_map(|group| group.mutations.iter().cloned()),
        )
        .collect()
}

/// The summary of every kept file's state once these groups are published.
fn summary(
    states: &BTreeMap<String, FileState>,
    groups: &[Group],
    kept: &std::collections::BTreeSet<&str>,
) -> Result<String> {
    let mut states = states.clone();
    // A retirement changes no kept file's state.
    for group in groups.iter().filter(|group| group.answer.is_some()) {
        for mutation in &group.mutations {
            if let Some(state) = records::state_of(mutation)? {
                states.insert(state.path.clone(), state);
            }
        }
    }
    states.retain(|path, _| kept.contains(path.as_str()));
    records::summary_sha256(&states)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(content: &str, status: Status, job: &str) -> FileState {
        FileState {
            path: "src/lib.rs".into(),
            content_sha256: content.into(),
            language: "rust".into(),
            parser_revision: "r1".into(),
            chunk_bytes: CHUNK_BYTES,
            job_id: job.into(),
            status,
            reason: String::new(),
            facts_sha256: String::new(),
            symbols: 0,
            spans: 0,
            chunks: 0,
            nodes: 0,
            error_nodes: 0,
        }
    }

    #[test]
    fn a_final_answer_for_the_same_content_is_not_parsed_again() {
        let wanted = state("c1", Status::Lost, "job-2");
        assert!(!settled(None, &wanted), "a new file is parsed");
        assert!(settled(
            Some(&state("c1", Status::Parsed, "job-1")),
            &wanted
        ));
        assert!(settled(
            Some(&state("c1", Status::Rejected, "job-1")),
            &wanted
        ));
        assert!(
            !settled(Some(&state("c0", Status::Parsed, "job-1")), &wanted),
            "an edit is parsed"
        );
    }

    #[test]
    fn a_lost_file_waits_for_the_next_job_instead_of_looping() {
        let wanted = state("c1", Status::Lost, "job-2");
        assert!(settled(Some(&state("c1", Status::Lost, "job-2")), &wanted));
        assert!(!settled(Some(&state("c1", Status::Lost, "job-1")), &wanted));
    }
}
