#![allow(dead_code)]

#[path = "projected_read_model/interleaved_lifetimes.rs"]
mod interleaved_lifetimes;
pub fn run_interleaved_lifetime_matrix(physical: &str) {
    interleaved_lifetimes::run_interleaved_lifetime_matrix(physical);
}

use rrd_lsm::{
    CompactionBoundary, CompactionPolicy, Database, DatabaseOptions, Durability, Error,
    FailureMode, FlushBoundary, Mutation, ProjectedReadBudget, ProjectedReadEvidence,
    ProjectedReadOutcome, ProjectedReadProjection, ProjectedReadRange, ProjectedReadRequest,
    ProjectedReadResource, SegmentCompressionPolicy, SegmentIoMode, SegmentIoPolicy,
    SegmentRowGroupBudget, Snapshot, WriteBatch, WriteBoundary,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const OPERATION_WIDTH: usize = 6;
const MAX_ENCODED_BYTES: usize = 4_096;
const FAMILY_PREFIXES: [&[u8]; 8] = [
    b"audit/",
    b"edge/in/",
    b"edge/out/",
    b"record/",
    b"runtime/",
    b"scalar/",
    b"term/",
    b"vector/",
];
type ProjectedRows = Vec<(Vec<u8>, Option<Vec<u8>>)>;
type ProjectedCollection = (ProjectedRows, ProjectedReadEvidence);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioConfig {
    pub seed: u64,
    pub cases: usize,
    pub operations: usize,
    pub retained_root: Option<PathBuf>,
}

impl ScenarioConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.cases == 0 {
            return Err("cases must be greater than zero".into());
        }
        if self.operations == 0 {
            return Err("operations must be greater than zero".into());
        }
        let encoded = self
            .operations
            .checked_mul(OPERATION_WIDTH)
            .ok_or_else(|| "operation byte count overflow".to_owned())?;
        if encoded > MAX_ENCODED_BYTES {
            return Err(format!(
                "operations require {encoded} bytes, above the {MAX_ENCODED_BYTES}-byte limit"
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScenarioReport {
    pub cases: u64,
    pub operations: u64,
    pub writes: u64,
    pub mutations: u64,
    pub injected_failures: u64,
    pub flushes: u64,
    pub compactions: u64,
    pub reopens: u64,
    pub garbage_collections: u64,
    pub projected_reads: u64,
    pub emitted_rows: u64,
    pub emitted_batches: u64,
    pub cancellations: u64,
    pub resource_denials: u64,
    pub verification_rounds: u64,
}

impl ScenarioReport {
    fn merge(&mut self, other: &Self) {
        self.cases += other.cases;
        self.operations += other.operations;
        self.writes += other.writes;
        self.mutations += other.mutations;
        self.injected_failures += other.injected_failures;
        self.flushes += other.flushes;
        self.compactions += other.compactions;
        self.reopens += other.reopens;
        self.garbage_collections += other.garbage_collections;
        self.projected_reads += other.projected_reads;
        self.emitted_rows += other.emitted_rows;
        self.emitted_batches += other.emitted_batches;
        self.cancellations += other.cancellations;
        self.resource_denials += other.resource_denials;
        self.verification_rounds += other.verification_rounds;
    }
}

#[derive(Debug, Clone)]
struct ModelVersion {
    sequence: u64,
    value: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Default)]
struct Model {
    histories: BTreeMap<Vec<u8>, Vec<ModelVersion>>,
    current_sequence: u64,
}

impl Model {
    fn apply(&mut self, mutations: &[Mutation], first_sequence: u64) {
        assert_eq!(
            first_sequence,
            self.current_sequence + 1,
            "model received a non-contiguous sequence"
        );
        for (offset, mutation) in mutations.iter().enumerate() {
            let sequence = first_sequence + offset as u64;
            let (key, value) = match mutation {
                Mutation::Put { key, value } => (key.clone(), Some(value.clone())),
                Mutation::Delete { key } => (key.clone(), None),
            };
            self.histories
                .entry(key)
                .or_default()
                .push(ModelVersion { sequence, value });
            self.current_sequence = sequence;
        }
    }

    fn get(&self, key: &[u8], sequence: u64) -> Option<Vec<u8>> {
        self.histories.get(key).and_then(|versions| {
            versions
                .iter()
                .rev()
                .find(|version| version.sequence <= sequence)
                .and_then(|version| version.value.clone())
        })
    }

    fn scan(&self, start: &[u8], end: Option<&[u8]>, sequence: u64) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.histories
            .keys()
            .filter(|key| key.as_slice() >= start && end.is_none_or(|end| key.as_slice() < end))
            .filter_map(|key| self.get(key, sequence).map(|value| (key.clone(), value)))
            .collect()
    }

    fn scan_ranges(&self, ranges: &[(Vec<u8>, Vec<u8>)], sequence: u64) -> Vec<(Vec<u8>, Vec<u8>)> {
        self.histories
            .keys()
            .filter(|key| {
                ranges.iter().any(|(start, end)| {
                    key.as_slice() >= start.as_slice() && key.as_slice() < end.as_slice()
                })
            })
            .filter_map(|key| self.get(key, sequence).map(|value| (key.clone(), value)))
            .collect()
    }
}

struct Scenario {
    root: PathBuf,
    options: DatabaseOptions,
    database: Option<Database>,
    model: Model,
    retained_snapshots: Vec<Snapshot>,
    at: u64,
    report: ScenarioReport,
}

impl Scenario {
    fn create(root: &Path, selector: u8, segment_compression: SegmentCompressionPolicy) -> Self {
        let row_group_rows = usize::from(selector % 4) + 1;
        let options = DatabaseOptions {
            page_cache_bytes: 16 * 1024,
            segment_io: SegmentIoPolicy {
                mode: SegmentIoMode::Bounded,
                allow_fallback: false,
                max_request_bytes: 4 * 1024,
                max_open_files: 128,
            },
            segment_row_group_budget: SegmentRowGroupBudget {
                max_rows: row_group_rows,
                target_bytes: 96 * row_group_rows,
            },
            segment_compression,
            compaction: CompactionPolicy {
                l0_compaction_trigger: 4,
                max_input_segments: 16,
                target_segment_bytes: 2 * 1024,
                max_level: 6,
            },
            ..DatabaseOptions::default()
        };
        let database = Database::create_with_options(root, options).unwrap();
        let mut scenario = Self {
            root: root.to_owned(),
            options,
            database: Some(database),
            model: Model::default(),
            retained_snapshots: Vec::new(),
            at: 1,
            report: ScenarioReport {
                cases: 1,
                ..ScenarioReport::default()
            },
        };
        scenario.seed_families(selector);
        scenario
    }

    fn database(&self) -> &Database {
        self.database.as_ref().expect("database is open")
    }

    fn database_mut(&mut self) -> &mut Database {
        self.database.as_mut().expect("database is open")
    }

    fn next_at(&mut self) -> u64 {
        let at = self.at;
        self.at += 1;
        at
    }

    fn seed_families(&mut self, selector: u8) {
        let mutations = FAMILY_PREFIXES
            .iter()
            .enumerate()
            .map(|(family, _)| Mutation::Put {
                key: key(family, usize::from(selector % 5)),
                value: value(selector, family as u8, family as u8),
            })
            .collect::<Vec<_>>();
        self.commit(mutations);
        self.flush();
        self.verify_all("seed");
    }

    fn commit(&mut self, mutations: Vec<Mutation>) {
        let expected_first = self.model.current_sequence + 1;
        let count = mutations.len() as u64;
        let batch = WriteBatch::new(mutations.clone()).unwrap();
        let receipt = self
            .database_mut()
            .write_owned(batch, Durability::Authoritative)
            .unwrap();
        assert_eq!(receipt.first_sequence, expected_first);
        assert_eq!(receipt.last_sequence, expected_first + count - 1);
        self.model.apply(&mutations, receipt.first_sequence);
        self.retained_snapshots.push(Snapshot {
            sequence: receipt.last_sequence,
        });
        self.trim_snapshots();
        self.report.writes += 1;
        self.report.mutations += count;
    }

    fn trim_snapshots(&mut self) {
        self.retained_snapshots
            .sort_by_key(|snapshot| snapshot.sequence);
        self.retained_snapshots
            .dedup_by_key(|snapshot| snapshot.sequence);
        if self.retained_snapshots.len() > 5 {
            let last = self.retained_snapshots.len() - 1;
            let retained = [
                self.retained_snapshots[0],
                self.retained_snapshots[last / 2],
                self.retained_snapshots[last],
            ];
            self.retained_snapshots = retained.to_vec();
            self.retained_snapshots
                .dedup_by_key(|snapshot| snapshot.sequence);
        }
    }

    fn flush(&mut self) {
        let at = self.next_at();
        if self.database_mut().flush_memtable(at).unwrap().is_some() {
            self.report.flushes += 1;
        }
    }

    fn compact(&mut self) {
        let protected = self.protected_snapshots();
        let at = self.next_at();
        if self
            .database_mut()
            .compact(&protected, at)
            .unwrap()
            .is_some()
        {
            self.report.compactions += 1;
            self.retained_snapshots = protected;
            self.trim_snapshots();
        }
    }

    fn protected_snapshots(&self) -> Vec<Snapshot> {
        let mut protected = self.retained_snapshots.clone();
        let current = Snapshot {
            sequence: self.model.current_sequence,
        };
        if !protected.contains(&current) {
            protected.push(current);
        }
        protected.sort_by_key(|snapshot| snapshot.sequence);
        protected.dedup_by_key(|snapshot| snapshot.sequence);
        if protected.len() > 3 {
            let last = protected.len() - 1;
            vec![protected[0], protected[last / 2], protected[last]]
        } else {
            protected
        }
    }

    fn reopen(&mut self) {
        drop(self.database.take());
        self.database = Some(Database::open_with_options(&self.root, self.options).unwrap());
        self.report.reopens += 1;
    }

    fn garbage_collect(&mut self) {
        self.database().garbage_collect().unwrap();
        self.report.garbage_collections += 1;
    }

    fn verify_all(&mut self, phase: &str) {
        let snapshots = self.retained_snapshots.clone();
        for snapshot in snapshots {
            self.verify_snapshot(snapshot, phase);
        }
        self.verify_snapshot(
            Snapshot {
                sequence: self.model.current_sequence,
            },
            phase,
        );
    }

    fn verify_snapshot(&mut self, snapshot: Snapshot, phase: &str) {
        assert_eq!(
            self.database().snapshot().sequence,
            self.model.current_sequence,
            "{phase}: database/model current sequence differs"
        );
        for key in self.model.histories.keys() {
            assert_eq!(
                self.database().get(key, snapshot).unwrap(),
                self.model.get(key, snapshot.sequence),
                "{phase}: point value differs at sequence {} for key {key:?}",
                snapshot.sequence
            );
        }
        assert_eq!(
            self.database().get(b"absent/key", snapshot).unwrap(),
            None,
            "{phase}: absent point read became visible"
        );
        let expected = self.model.scan(&[], None, snapshot.sequence);
        assert_eq!(
            self.database().scan(&[], None, snapshot).unwrap(),
            expected,
            "{phase}: complete range differs at sequence {}",
            snapshot.sequence
        );
        let ranges = selected_ranges();
        let expected_ranges = self.model.scan_ranges(&ranges, snapshot.sequence);
        assert_eq!(
            self.database().scan_ranges(&ranges, snapshot).unwrap(),
            expected_ranges,
            "{phase}: disjoint ranges differ at sequence {}",
            snapshot.sequence
        );
        self.verify_projected(
            snapshot,
            ProjectedReadProjection::KeyValue,
            vec![ProjectedReadRange::all()],
            expected
                .iter()
                .map(|(key, value)| (key.clone(), Some(value.clone())))
                .collect(),
            phase,
        );
        self.verify_projected(
            snapshot,
            ProjectedReadProjection::Key,
            ranges
                .iter()
                .map(|(start, end)| {
                    ProjectedReadRange::new(start.clone(), Some(end.clone())).unwrap()
                })
                .collect(),
            expected_ranges
                .iter()
                .map(|(key, _)| (key.clone(), None))
                .collect(),
            phase,
        );
        self.report.verification_rounds += 1;
    }

    fn verify_projected(
        &mut self,
        snapshot: Snapshot,
        projection: ProjectedReadProjection,
        ranges: Vec<ProjectedReadRange>,
        expected: Vec<(Vec<u8>, Option<Vec<u8>>)>,
        phase: &str,
    ) {
        let batch_rows = usize::from((snapshot.sequence % 3) as u8) + 1;
        let request = ProjectedReadRequest {
            ranges,
            projection,
            snapshot,
            budget: ProjectedReadBudget {
                max_batch_rows: batch_rows,
                ..ProjectedReadBudget::default()
            },
        };
        let (actual, evidence) = collect_projected(self.database(), request);
        assert_eq!(
            actual, expected,
            "{phase}: projected {projection:?} rows differ at sequence {}",
            snapshot.sequence
        );
        assert_eq!(evidence.outcome, ProjectedReadOutcome::Completed);
        assert!(evidence.peak_batch_rows <= batch_rows as u64);
        if projection == ProjectedReadProjection::Key {
            assert_eq!(evidence.value_offset_page_requests, 0);
            assert_eq!(evidence.value_data_page_requests, 0);
        }
        self.report.projected_reads += 1;
        self.report.emitted_rows += evidence.emitted_rows;
        self.report.emitted_batches += evidence.emitted_batches;
    }

    fn execute(&mut self, operation: usize, bytes: &[u8]) {
        let opcode = bytes[0] % 16;
        match opcode {
            0 => self.commit(vec![generated_mutation(bytes, operation, false)]),
            1 => self.commit(vec![generated_mutation(bytes, operation, true)]),
            2 => self.commit(multi_family_mutations(bytes, operation)),
            3 => self.flush(),
            4 => self.compact(),
            5 => self.reopen(),
            6 => self.verify_all("generated verification"),
            7 => self.pinned_interleaving(bytes, operation),
            8 => self.cancel_and_drop(),
            9 => self.write_failure(bytes, operation),
            10 => self.flush_failure(bytes, operation),
            11 => self.compaction_failure(bytes, operation),
            12 => self.garbage_collect(),
            13 => {
                self.retained_snapshots.push(Snapshot {
                    sequence: self.model.current_sequence,
                });
                self.trim_snapshots();
            }
            14 => self.resource_denial(),
            15 => self.verify_snapshot(
                Snapshot {
                    sequence: self.model.current_sequence,
                },
                "generated current verification",
            ),
            _ => unreachable!(),
        }
        self.report.operations += 1;
        self.verify_current_points(operation);
    }

    fn verify_current_points(&mut self, operation: usize) {
        let snapshot = Snapshot {
            sequence: self.model.current_sequence,
        };
        assert_eq!(self.database().snapshot(), snapshot);
        for key in self.model.histories.keys() {
            assert_eq!(
                self.database().get(key, snapshot).unwrap(),
                self.model.get(key, snapshot.sequence),
                "operation {operation}: current point differs for {key:?}"
            );
        }
    }

    fn pinned_interleaving(&mut self, bytes: &[u8], operation: usize) {
        let snapshot = Snapshot {
            sequence: self.model.current_sequence,
        };
        let expected = self
            .model
            .scan(&[], None, snapshot.sequence)
            .into_iter()
            .map(|(key, value)| (key, Some(value)))
            .collect::<Vec<_>>();
        let mut stream = self
            .database()
            .begin_projected_read(ProjectedReadRequest {
                ranges: vec![ProjectedReadRange::all()],
                projection: ProjectedReadProjection::KeyValue,
                snapshot,
                budget: ProjectedReadBudget {
                    max_batch_rows: 1,
                    ..ProjectedReadBudget::default()
                },
            })
            .unwrap();
        self.commit(vec![generated_mutation(bytes, operation, false)]);
        self.flush();
        let protected = vec![
            snapshot,
            Snapshot {
                sequence: self.model.current_sequence,
            },
        ];
        let at = self.next_at();
        if self
            .database_mut()
            .compact(&protected, at)
            .unwrap()
            .is_some()
        {
            self.report.compactions += 1;
            self.retained_snapshots = protected;
            self.trim_snapshots();
        }
        self.garbage_collect();
        let (actual, evidence) = collect_existing_stream(&mut stream);
        assert_eq!(actual, expected, "pinned projected view changed");
        assert_eq!(evidence.snapshot_sequence, snapshot.sequence);
        assert_eq!(evidence.outcome, ProjectedReadOutcome::Completed);
        self.report.projected_reads += 1;
        self.report.emitted_rows += evidence.emitted_rows;
        self.report.emitted_batches += evidence.emitted_batches;
        drop(stream);
        self.garbage_collect();
    }

    fn cancel_and_drop(&mut self) {
        let snapshot = Snapshot {
            sequence: self.model.current_sequence,
        };
        let request = ProjectedReadRequest {
            ranges: vec![ProjectedReadRange::all()],
            projection: ProjectedReadProjection::Key,
            snapshot,
            budget: ProjectedReadBudget {
                max_batch_rows: 1,
                ..ProjectedReadBudget::default()
            },
        };
        let mut stream = self
            .database()
            .begin_projected_read(request.clone())
            .unwrap();
        let _ = stream.next_batch().unwrap();
        stream.cancel().unwrap();
        assert_eq!(stream.evidence().outcome, ProjectedReadOutcome::Cancelled);
        assert!(stream.next_batch().unwrap().is_none());
        self.report.cancellations += 1;

        let mut one_view = request;
        one_view.budget.max_active_views = 1;
        let stream = self
            .database()
            .begin_projected_read(one_view.clone())
            .unwrap();
        drop(stream);
        drop(self.database().begin_projected_read(one_view).unwrap());
    }

    fn write_failure(&mut self, bytes: &[u8], operation: usize) {
        let boundaries = [
            WriteBoundary::Prepared,
            WriteBoundary::WalAppended,
            WriteBoundary::WalSynced,
            WriteBoundary::Visible,
        ];
        let boundary = boundaries[usize::from(bytes[1]) % boundaries.len()];
        let mode = failure_mode(bytes[2]);
        let mutations = if bytes[3].is_multiple_of(2) {
            multi_family_mutations(bytes, operation)
        } else {
            vec![generated_mutation(bytes, operation, false)]
        };
        let expected_first = self.model.current_sequence + 1;
        let error = self
            .database_mut()
            .write_owned_with_failure(
                WriteBatch::new(mutations.clone()).unwrap(),
                if bytes[4].is_multiple_of(2) {
                    Durability::Authoritative
                } else {
                    Durability::Buffered
                },
                boundary,
                mode,
            )
            .unwrap_err();
        assert_injected(error, write_boundary_name(boundary), mode);
        if boundary != WriteBoundary::Prepared {
            self.model.apply(&mutations, expected_first);
            self.retained_snapshots.push(Snapshot {
                sequence: self.model.current_sequence,
            });
            self.trim_snapshots();
        }
        self.report.injected_failures += 1;
        self.reopen();
    }

    fn flush_failure(&mut self, bytes: &[u8], operation: usize) {
        self.commit(vec![generated_mutation(bytes, operation, false)]);
        let boundaries = [
            FlushBoundary::WalSynced,
            FlushBoundary::SegmentSynced,
            FlushBoundary::SuccessorWalSynced,
            FlushBoundary::ManifestPublished,
        ];
        let boundary = boundaries[usize::from(bytes[1]) % boundaries.len()];
        let mode = failure_mode(bytes[2]);
        let at = self.next_at();
        let error = self
            .database_mut()
            .flush_memtable_with_failure(at, boundary, mode)
            .unwrap_err();
        assert_injected(error, flush_boundary_name(boundary), mode);
        self.report.injected_failures += 1;
        self.reopen();
    }

    fn compaction_failure(&mut self, bytes: &[u8], operation: usize) {
        let shared_family = usize::from(bytes[1]) % FAMILY_PREFIXES.len();
        for revision in 0..2u8 {
            self.commit(vec![Mutation::Put {
                key: key(shared_family, usize::from(bytes[2] % 4)),
                value: value(bytes[3], operation as u8, revision),
            }]);
            self.flush();
        }
        let boundaries = [
            CompactionBoundary::SegmentSynced,
            CompactionBoundary::ManifestPublished,
        ];
        let boundary = boundaries[usize::from(bytes[4]) % boundaries.len()];
        let mode = failure_mode(bytes[5]);
        let protected = self.protected_snapshots();
        let at = self.next_at();
        let error = self
            .database_mut()
            .compact_with_failure(&protected, at, boundary, mode)
            .unwrap_err();
        assert_injected(error, compaction_boundary_name(boundary), mode);
        if boundary == CompactionBoundary::ManifestPublished {
            self.retained_snapshots = protected;
            self.trim_snapshots();
        }
        self.report.injected_failures += 1;
        self.reopen();
    }

    fn resource_denial(&mut self) {
        let snapshot = Snapshot {
            sequence: self.model.current_sequence,
        };
        let mut output_limited = self
            .database()
            .begin_projected_read(ProjectedReadRequest {
                ranges: vec![ProjectedReadRange::all()],
                projection: ProjectedReadProjection::KeyValue,
                snapshot,
                budget: ProjectedReadBudget {
                    max_output_rows: 1,
                    max_batch_rows: 1,
                    ..ProjectedReadBudget::default()
                },
            })
            .unwrap();
        assert_eq!(output_limited.next_batch().unwrap().unwrap().len(), 1);
        assert!(matches!(
            output_limited.next_batch(),
            Err(Error::ProjectedReadLimit {
                resource: ProjectedReadResource::OutputRows,
                ..
            })
        ));
        assert_eq!(
            output_limited.evidence().outcome,
            ProjectedReadOutcome::Failed
        );
        assert!(output_limited.next_batch().unwrap().is_none());
        self.report.resource_denials += 1;

        let mut page_limited = self
            .database()
            .begin_projected_read(ProjectedReadRequest {
                ranges: vec![ProjectedReadRange::all()],
                projection: ProjectedReadProjection::Key,
                snapshot,
                budget: ProjectedReadBudget {
                    max_page_requests: 1,
                    ..ProjectedReadBudget::default()
                },
            })
            .unwrap();
        assert!(matches!(
            page_limited.next_batch(),
            Err(Error::ProjectedReadLimit {
                resource: ProjectedReadResource::PageRequests,
                ..
            })
        ));
        self.report.resource_denials += 1;
        drop(page_limited);

        let request = ProjectedReadRequest {
            ranges: vec![ProjectedReadRange::all()],
            projection: ProjectedReadProjection::Key,
            snapshot,
            budget: ProjectedReadBudget {
                max_active_views: 1,
                ..ProjectedReadBudget::default()
            },
        };
        drop(self.database().begin_projected_read(request).unwrap());
    }
}

pub fn run_encoded_scenario(seed: u64, input: &[u8]) -> ScenarioReport {
    run_encoded_scenario_with_policy(seed, input, SegmentCompressionPolicy::default())
}

pub fn run_encoded_scenario_with_policy(
    seed: u64,
    input: &[u8],
    segment_compression: SegmentCompressionPolicy,
) -> ScenarioReport {
    assert!(
        input.len() <= MAX_ENCODED_BYTES,
        "encoded scenario exceeds {MAX_ENCODED_BYTES} bytes"
    );
    let temporary = tempfile::tempdir().unwrap();
    run_encoded_scenario_at_with_policy(seed, input, temporary.path(), segment_compression)
}

pub fn run_encoded_scenario_at(seed: u64, input: &[u8], root: &Path) -> ScenarioReport {
    run_encoded_scenario_at_with_policy(seed, input, root, SegmentCompressionPolicy::default())
}

pub fn run_encoded_scenario_at_with_policy(
    seed: u64,
    input: &[u8],
    root: &Path,
    segment_compression: SegmentCompressionPolicy,
) -> ScenarioReport {
    let replay = replay_coordinate(seed, input);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let selector = input.first().copied().unwrap_or(seed as u8);
        let mut scenario = Scenario::create(root, selector, segment_compression);
        for (operation, chunk) in input.chunks(OPERATION_WIDTH).enumerate() {
            let mut padded = [0u8; OPERATION_WIDTH];
            padded[..chunk.len()].copy_from_slice(chunk);
            scenario.execute(operation, &padded);
        }
        scenario.verify_all("final");
        scenario.report
    }));
    match outcome {
        Ok(report) => report,
        Err(payload) => {
            let reason = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("non-string panic");
            panic!("rrflowKV scenario failed at {replay}: {reason}");
        }
    }
}

pub fn run_seeded_scenarios(config: &ScenarioConfig) -> Result<ScenarioReport, String> {
    config.validate()?;
    if let Some(root) = &config.retained_root {
        std::fs::create_dir_all(root)
            .map_err(|error| format!("cannot create retained root {}: {error}", root.display()))?;
    }
    let mut total = ScenarioReport::default();
    for case in 0..config.cases {
        let seed = config
            .seed
            .wrapping_add((case as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
        let program = generated_program(seed, config.operations);
        let report = if let Some(root) = &config.retained_root {
            let case_root = root.join(format!("case-{case:06}-{seed:016x}"));
            run_encoded_scenario_at(seed, &program, &case_root)
        } else {
            run_encoded_scenario(seed, &program)
        };
        total.merge(&report);
    }
    Ok(total)
}

pub fn generated_program(seed: u64, operations: usize) -> Vec<u8> {
    assert!(operations <= MAX_ENCODED_BYTES / OPERATION_WIDTH);
    let mut state = seed;
    let mut output = Vec::with_capacity(operations * OPERATION_WIDTH);
    for operation in 0..operations {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let mixed = state.wrapping_add((operation as u64).rotate_left(19));
        output.extend_from_slice(&mixed.to_le_bytes()[..OPERATION_WIDTH]);
    }
    output
}

pub fn directed_failure_program() -> Vec<u8> {
    let mut output = Vec::new();
    for boundary in 0..4u8 {
        output.extend_from_slice(&[9, boundary, boundary % 2, 0, boundary, 1]);
    }
    for boundary in 0..4u8 {
        output.extend_from_slice(&[10, boundary, (boundary + 1) % 2, 2, 3, 4]);
    }
    for boundary in 0..2u8 {
        output.extend_from_slice(&[11, 3, 1, 9, boundary, boundary % 2]);
    }
    output
}

pub fn directed_lifetime_program() -> Vec<u8> {
    vec![
        0, 1, 2, 3, 4, 5, 7, 2, 4, 6, 8, 10, 8, 0, 1, 2, 3, 4, 14, 9, 8, 7, 6, 5, 12, 4, 3, 2, 1,
        0, 5, 1, 2, 3, 4, 5,
    ]
}

pub fn replay_coordinate(seed: u64, input: &[u8]) -> String {
    let mut digest = 0xcbf2_9ce4_8422_2325u64;
    let mut encoded = String::with_capacity(input.len() * 2);
    for byte in input {
        digest ^= u64::from(*byte);
        digest = digest.wrapping_mul(0x0000_0100_0000_01b3);
        use std::fmt::Write as _;
        write!(&mut encoded, "{byte:02x}").expect("writing into String cannot fail");
    }
    format!(
        "seed={seed:#018x}, input_bytes={}, input_digest={digest:016x}, input_hex={encoded}",
        input.len()
    )
}

fn key(family: usize, identifier: usize) -> Vec<u8> {
    let prefix = FAMILY_PREFIXES[family % FAMILY_PREFIXES.len()];
    let mut output = Vec::with_capacity(prefix.len() + 4);
    output.extend_from_slice(prefix);
    output.extend_from_slice(&(identifier as u16).to_be_bytes());
    output.push(match identifier % 3 {
        0 => 0,
        1 => 0xff,
        _ => (identifier as u8).wrapping_mul(29),
    });
    output
}

fn value(selector: u8, operation: u8, offset: u8) -> Vec<u8> {
    let length = usize::from(selector % 48) + 1;
    (0..length)
        .map(|index| {
            selector
                .wrapping_mul(17)
                .wrapping_add(operation.rotate_left(2))
                .wrapping_add(offset)
                .wrapping_add(index as u8)
        })
        .collect()
}

fn generated_mutation(bytes: &[u8], operation: usize, force_delete: bool) -> Mutation {
    let family = usize::from(bytes[1]) % FAMILY_PREFIXES.len();
    let identifier = (usize::from(bytes[2]) + operation) % 16;
    let key = key(family, identifier);
    if force_delete || bytes[3].is_multiple_of(5) {
        Mutation::Delete { key }
    } else {
        Mutation::Put {
            key,
            value: value(bytes[4], operation as u8, bytes[5]),
        }
    }
}

fn multi_family_mutations(bytes: &[u8], operation: usize) -> Vec<Mutation> {
    FAMILY_PREFIXES
        .iter()
        .enumerate()
        .map(|(family, _)| {
            let identifier = (usize::from(bytes[(family % 5) + 1]) + operation) % 12;
            let key = key(family, identifier);
            if (bytes[5].wrapping_add(family as u8)).is_multiple_of(7) {
                Mutation::Delete { key }
            } else {
                Mutation::Put {
                    key,
                    value: value(bytes[4], operation as u8, family as u8),
                }
            }
        })
        .collect()
}

fn selected_ranges() -> Vec<(Vec<u8>, Vec<u8>)> {
    [0usize, 3, 6]
        .into_iter()
        .map(|family| {
            let start = FAMILY_PREFIXES[family].to_vec();
            let mut end = start.clone();
            let last = end.last_mut().expect("family prefix is non-empty");
            *last += 1;
            (start, end)
        })
        .collect()
}

fn collect_projected(database: &Database, request: ProjectedReadRequest) -> ProjectedCollection {
    let mut stream = database.begin_projected_read(request).unwrap();
    collect_existing_stream(&mut stream)
}

fn collect_existing_stream(stream: &mut rrd_lsm::ProjectedReadStream) -> ProjectedCollection {
    let mut rows = Vec::new();
    while let Some(batch) = stream.next_batch().unwrap() {
        for row in 0..batch.len() {
            rows.push((
                batch.key(row).unwrap().to_vec(),
                batch.value(row).map(<[u8]>::to_vec),
            ));
        }
    }
    assert!(stream.next_batch().unwrap().is_none());
    for adjacent in rows.windows(2) {
        assert!(
            adjacent[0].0 < adjacent[1].0,
            "projected rows are not strict"
        );
    }
    (rows, stream.evidence().clone())
}

fn failure_mode(selector: u8) -> FailureMode {
    if selector.is_multiple_of(2) {
        FailureMode::Crash
    } else {
        FailureMode::StorageFull
    }
}

fn assert_injected(error: Error, boundary: &'static str, mode: FailureMode) {
    let expected_mode = match mode {
        FailureMode::Crash => "crash",
        FailureMode::StorageFull => "storage-full",
    };
    assert!(matches!(
        error,
        Error::InjectedFailure {
            mode: actual_mode,
            boundary: actual_boundary,
        } if actual_mode == expected_mode && actual_boundary == boundary
    ));
}

fn write_boundary_name(boundary: WriteBoundary) -> &'static str {
    match boundary {
        WriteBoundary::Prepared => "write.prepared",
        WriteBoundary::WalAppended => "write.wal_appended",
        WriteBoundary::WalSynced => "write.wal_synced",
        WriteBoundary::Visible => "write.visible",
    }
}

fn flush_boundary_name(boundary: FlushBoundary) -> &'static str {
    match boundary {
        FlushBoundary::WalSynced => "flush.wal_synced",
        FlushBoundary::SegmentSynced => "flush.segment_synced",
        FlushBoundary::SuccessorWalSynced => "flush.successor_wal_synced",
        FlushBoundary::ManifestPublished => "flush.manifest_published",
    }
}

fn compaction_boundary_name(boundary: CompactionBoundary) -> &'static str {
    match boundary {
        CompactionBoundary::SegmentSynced => "compaction.segment_synced",
        CompactionBoundary::ManifestPublished => "compaction.manifest_published",
    }
}
