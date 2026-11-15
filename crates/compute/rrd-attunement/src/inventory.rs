//! Deterministic inventory computation over engine-authorized observations.
//!
//! This module never opens a path or commits a record. A complete proposal says
//! only that its supplied enumeration completed without reported errors. The
//! engine must revalidate the observed source boundary and atomically publish
//! entries, containment, change set and checkpoint before calling it a snapshot.

use rrd_core::digest::{sha256_hex, Sha256};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceScope {
    pub engram: String,
    pub project: String,
    /// Logical coordinate, never a host-absolute path.
    pub root: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revisions {
    pub policy_sha256: String,
    pub walker: String,
    pub detector: String,
}

/// Compute admission limits; filesystem duration, cancellation and descriptor
/// limits are additionally enforced by the engine before/while supplying input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Limits {
    /// Includes the implicit root directory.
    pub entries: u64,
    pub depth: u64,
    pub path_bytes: u64,
    pub metadata_bytes: u64,
    pub file_bytes: u64,
    pub content_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileObservation {
    pub bytes: u64,
    pub executable: bool,
    /// Engine-owned stable file identity/change observation, not just mtime.
    /// Reuse is sound only if the engine's observation detects replacements.
    pub revision: String,
    /// Digest of versioned classification/detector evidence, without raw content.
    pub classification_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EntryKind {
    Directory,
    File {
        observation: FileObservation,
        content_sha256: String,
    },
    /// Policy exclusion decisions retain a rule identity, never excluded bytes.
    Excluded {
        rule: String,
    },
    /// The engine records a non-followed leaf without exposing its target.
    Symlink {
        rule: String,
    },
    Mount {
        rule: String,
    },
    Error {
        class: ErrorClass,
        retryable: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorClass {
    Unreadable,
    Vanished,
    Replaced,
    ChangedDuringRead,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub path: String,
    pub identity: String,
    pub parent: Option<String>,
    pub kind: EntryKind,
    pub subtree_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntryChange {
    pub path: String,
    pub identity: String,
    pub before_sha256: Option<String>,
    pub after_sha256: Option<String>,
}

/// A unique equal-content add/remove pair is evidence for a candidate only.
/// No identity is reassigned and no OS rename event is assumed authoritative.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameCandidate {
    pub removed_identity: String,
    pub added_identity: String,
    pub content_sha256: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changes {
    pub added: Vec<EntryChange>,
    pub modified: Vec<EntryChange>,
    pub removed: Vec<EntryChange>,
    pub rename_candidates: Vec<RenameCandidate>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Accounting {
    pub content_bytes: u64,
    pub reused_files: u64,
    pub errors: u64,
    pub rejected_observations: u64,
}

/// Constructed only by the bounded builder. It is a proposal, not a receipt or
/// read stamp. Private fields prevent callers from assigning completeness or
/// replacing its digest without rebuilding and validating all observations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Proposal {
    format_version: u32,
    scope: SourceScope,
    revisions: Revisions,
    limits: Limits,
    previous_sha256: Option<String>,
    complete: bool,
    root_sha256: String,
    entries: Vec<Entry>,
    changes: Changes,
    accounting: Accounting,
    proposal_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommittedProposal {
    format_version: u32,
    scope: SourceScope,
    revisions: Revisions,
    limits: Limits,
    previous_sha256: Option<String>,
    complete: bool,
    root_sha256: String,
    entries: Vec<Entry>,
    changes: Changes,
    accounting: Accounting,
    proposal_sha256: String,
}

impl Proposal {
    /// Reconstruct bounded data whose expected identity the ENGINE obtained
    /// from its committed checkpoint and authenticated runtime read. A checksum
    /// supplied alongside arbitrary bytes is not provenance or authorization.
    /// This validates structure/Merkle identities; historical change provenance
    /// remains bound by the expected committed proposal digest.
    pub fn from_committed_bytes(
        bytes: &[u8],
        expected_sha256: &str,
        max_bytes: usize,
    ) -> Result<Self> {
        if bytes.len() > max_bytes || !digest(expected_sha256) {
            return Err(InventoryError::InvalidCommittedProposal);
        }
        let wire: CommittedProposal =
            serde_json::from_slice(bytes).map_err(|_| InventoryError::Encoding)?;
        let mut result = Self {
            format_version: wire.format_version,
            scope: wire.scope,
            revisions: wire.revisions,
            limits: wire.limits,
            previous_sha256: wire.previous_sha256,
            complete: wire.complete,
            root_sha256: wire.root_sha256,
            entries: wire.entries,
            changes: wire.changes,
            accounting: wire.accounting,
            proposal_sha256: String::new(),
        };
        if result.format_version != FORMAT_VERSION
            || !result.complete
            || result.accounting.errors != 0
            || result.accounting.rejected_observations != 0
            || result.previous_sha256.as_ref().is_some_and(|s| !digest(s))
            || result.accounting.content_bytes > result.limits.content_bytes
            || wire.proposal_sha256 != expected_sha256
            || hash(&("rrflow-source-proposal-v1", &result))? != expected_sha256
        {
            return Err(InventoryError::InvalidCommittedProposal);
        }
        let mut rebuilt = Builder::new(
            result.scope.clone(),
            result.revisions.clone(),
            result.limits.clone(),
            None,
        )?;
        if result
            .entries
            .first()
            .is_none_or(|e| !e.path.is_empty() || e.kind != EntryKind::Directory)
        {
            return Err(InventoryError::InvalidCommittedProposal);
        }
        let mut files = 0u64;
        let mut previous = "";
        for entry in result.entries.iter().skip(1) {
            if entry.path.as_str() <= previous {
                return Err(InventoryError::InvalidCommittedProposal);
            }
            previous = &entry.path;
            match &entry.kind {
                EntryKind::Directory => rebuilt.directory(&entry.path)?,
                EntryKind::File {
                    observation,
                    content_sha256,
                } => {
                    rebuilt.admit_path(&entry.path)?;
                    if !digest(content_sha256)
                        || observation.bytes > result.limits.file_bytes
                        || observation.revision.is_empty()
                        || observation.revision.len() as u64 > result.limits.metadata_bytes
                        || observation.revision.chars().any(char::is_control)
                        || !digest(&observation.classification_sha256)
                    {
                        return Err(InventoryError::InvalidCommittedProposal);
                    }
                    rebuilt
                        .entries
                        .insert(entry.path.clone(), entry.kind.clone());
                    files += 1;
                }
                EntryKind::Error { .. } => return Err(InventoryError::InvalidCommittedProposal),
                other => rebuilt.leaf(&entry.path, other.clone())?,
            }
        }
        let checked = rebuilt.finish(true)?;
        if checked.entries != result.entries
            || checked.root_sha256 != result.root_sha256
            || result.accounting.reused_files > files
        {
            return Err(InventoryError::InvalidCommittedProposal);
        }
        result.proposal_sha256 = expected_sha256.into();
        Ok(result)
    }

    pub fn is_complete(&self) -> bool {
        self.complete
    }
    pub fn root_sha256(&self) -> &str {
        &self.root_sha256
    }
    pub fn proposal_sha256(&self) -> &str {
        &self.proposal_sha256
    }
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    pub fn revisions(&self) -> &Revisions {
        &self.revisions
    }
    pub fn scope(&self) -> &SourceScope {
        &self.scope
    }
    pub fn changes(&self) -> &Changes {
        &self.changes
    }
    pub fn accounting(&self) -> &Accounting {
        &self.accounting
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryError {
    InvalidScope,
    InvalidRevision,
    InvalidLimits,
    InvalidPath,
    DuplicatePath,
    MissingParent,
    InvalidParent,
    EntryLimit,
    ContentLimit,
    InvalidObservation,
    ContentLength,
    IncompletePrevious,
    ForeignPrevious,
    Encoding,
    ContentRead,
    InvalidCommittedProposal,
}

impl fmt::Display for InventoryError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(out, "inventory proposal rejected: {self:?}")
    }
}
impl std::error::Error for InventoryError {}
type Result<T> = std::result::Result<T, InventoryError>;

pub struct Builder<'a> {
    scope: SourceScope,
    revisions: Revisions,
    limits: Limits,
    prior: Option<&'a Proposal>,
    entries: BTreeMap<String, EntryKind>,
    accounting: Accounting,
}

impl<'a> Builder<'a> {
    pub fn new(
        scope: SourceScope,
        revisions: Revisions,
        limits: Limits,
        prior: Option<&'a Proposal>,
    ) -> Result<Self> {
        if [&scope.engram, &scope.project, &scope.root]
            .iter()
            .any(|s| !identifier(s))
        {
            return Err(InventoryError::InvalidScope);
        }
        if !digest(&revisions.policy_sha256)
            || !identifier(&revisions.walker)
            || !identifier(&revisions.detector)
        {
            return Err(InventoryError::InvalidRevision);
        }
        if limits.entries == 0
            || limits.depth == 0
            || limits.path_bytes == 0
            || limits.metadata_bytes == 0
            || limits.file_bytes > limits.content_bytes
        {
            return Err(InventoryError::InvalidLimits);
        }
        if let Some(previous) = prior {
            if !previous.complete {
                return Err(InventoryError::IncompletePrevious);
            }
            if previous.scope != scope {
                return Err(InventoryError::ForeignPrevious);
            }
        }
        Ok(Self {
            scope,
            revisions,
            limits,
            prior,
            entries: BTreeMap::from([(String::new(), EntryKind::Directory)]),
            accounting: Accounting::default(),
        })
    }

    fn admit_path(&self, path: &str) -> Result<()> {
        if path.is_empty()
            || path.len() as u64 > self.limits.path_bytes
            || path.contains(['\\', ':'])
            || path.chars().any(char::is_control)
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            || path.split('/').count() as u64 > self.limits.depth
        {
            return Err(InventoryError::InvalidPath);
        }
        if self.entries.contains_key(path) {
            return Err(InventoryError::DuplicatePath);
        }
        if self.entries.len() as u64 >= self.limits.entries {
            return Err(InventoryError::EntryLimit);
        }
        Ok(())
    }

    pub fn directory(&mut self, path: &str) -> Result<()> {
        let result = self.admit_path(path);
        self.record_rejection(&result);
        result?;
        self.entries.insert(path.into(), EntryKind::Directory);
        Ok(())
    }

    /// Supply lazy bounded content chunks only after the engine has admitted the
    /// file. Empty files supply no chunks; nonempty chunks must make progress.
    /// On exact reuse this never requests a chunk. On a read error, discard
    /// this builder or record an error leaf; never pass truncated content as EOF.
    pub fn file<I, B>(&mut self, path: &str, observation: FileObservation, chunks: I) -> Result<()>
    where
        I: IntoIterator<Item = B>,
        B: AsRef<[u8]>,
    {
        self.file_fallible(
            path,
            observation,
            chunks.into_iter().map(Ok::<B, std::convert::Infallible>),
            true,
        )
    }

    /// The engine supplies `reuse_verified` only after independently proving
    /// the native reuse boundary. Prior metadata equality alone is insufficient.
    /// A failed chunk is an error, never clean EOF or a publishable partial file.
    pub fn file_fallible<I, B, E>(
        &mut self,
        path: &str,
        observation: FileObservation,
        chunks: I,
        reuse_verified: bool,
    ) -> Result<()>
    where
        I: IntoIterator<Item = std::result::Result<B, E>>,
        B: AsRef<[u8]>,
    {
        let result = self.file_inner(path, observation, chunks, reuse_verified);
        self.record_rejection(&result);
        result
    }

    pub fn content_sha256(&self, path: &str) -> Option<&str> {
        match self.entries.get(path) {
            Some(EntryKind::File { content_sha256, .. }) => Some(content_sha256),
            _ => None,
        }
    }

    fn record_rejection(&mut self, result: &Result<()>) {
        if result.is_err() {
            self.accounting.rejected_observations =
                self.accounting.rejected_observations.saturating_add(1);
        }
    }

    fn file_inner<I, B, E>(
        &mut self,
        path: &str,
        observation: FileObservation,
        chunks: I,
        reuse_verified: bool,
    ) -> Result<()>
    where
        I: IntoIterator<Item = std::result::Result<B, E>>,
        B: AsRef<[u8]>,
    {
        self.admit_path(path)?;
        if observation.revision.is_empty()
            || observation.revision.len() as u64 > self.limits.metadata_bytes
            || observation.revision.chars().any(char::is_control)
            || !digest(&observation.classification_sha256)
        {
            return Err(InventoryError::InvalidObservation);
        }
        if observation.bytes > self.limits.file_bytes {
            return Err(InventoryError::ContentLimit);
        }
        let reused = self
            .prior
            .filter(|p| reuse_verified && p.revisions == self.revisions && p.limits == self.limits)
            .and_then(|p| {
                p.entries
                    .binary_search_by(|e| e.path.as_str().cmp(path))
                    .ok()
                    .map(|i| &p.entries[i])
            })
            .and_then(|entry| match &entry.kind {
                EntryKind::File {
                    observation: old,
                    content_sha256,
                } if *old == observation => Some(content_sha256.clone()),
                _ => None,
            });
        let content_sha256 = if let Some(digest) = reused {
            self.accounting.reused_files += 1;
            digest
        } else {
            self.accounting
                .content_bytes
                .checked_add(observation.bytes)
                .filter(|n| *n <= self.limits.content_bytes)
                .ok_or(InventoryError::ContentLimit)?;
            let mut length = 0u64;
            let mut hasher = Sha256::new();
            for chunk in chunks {
                let chunk = chunk.map_err(|_| InventoryError::ContentRead)?;
                let bytes = chunk.as_ref();
                // Empty files supply no chunks. Reject a stalled producer so
                // arbitrarily many empty chunks cannot evade the byte bound.
                if bytes.is_empty() {
                    return Err(InventoryError::ContentLength);
                }
                self.accounting.content_bytes = self
                    .accounting
                    .content_bytes
                    .checked_add(bytes.len() as u64)
                    .ok_or(InventoryError::ContentLimit)?;
                length = length
                    .checked_add(bytes.len() as u64)
                    .filter(|n| *n <= observation.bytes)
                    .ok_or(InventoryError::ContentLength)?;
                hasher.update(bytes);
            }
            if length != observation.bytes {
                return Err(InventoryError::ContentLength);
            }
            hasher.finalize_hex()
        };
        self.entries.insert(
            path.into(),
            EntryKind::File {
                observation,
                content_sha256,
            },
        );
        Ok(())
    }

    /// Only non-content leaves can enter here. File digests are computed above.
    pub fn leaf(&mut self, path: &str, kind: EntryKind) -> Result<()> {
        let result = self.leaf_inner(path, kind);
        self.record_rejection(&result);
        result
    }

    fn leaf_inner(&mut self, path: &str, kind: EntryKind) -> Result<()> {
        self.admit_path(path)?;
        match &kind {
            EntryKind::Excluded { rule }
            | EntryKind::Symlink { rule }
            | EntryKind::Mount { rule }
                if identifier(rule) => {}
            EntryKind::Error { .. } => {}
            _ => return Err(InventoryError::InvalidObservation),
        }
        if matches!(kind, EntryKind::Error { .. }) {
            self.accounting.errors += 1;
        }
        self.entries.insert(path.into(), kind);
        Ok(())
    }

    pub fn finish(self, enumeration_complete: bool) -> Result<Proposal> {
        let mut children = BTreeMap::<&str, Vec<&str>>::new();
        for path in self.entries.keys().filter(|p| !p.is_empty()) {
            let parent = parent(path);
            match self.entries.get(parent) {
                Some(EntryKind::Directory) => {}
                Some(_) => return Err(InventoryError::InvalidParent),
                None => return Err(InventoryError::MissingParent),
            }
            children.entry(parent).or_default().push(path);
        }
        let mut built = BTreeMap::<String, Entry>::new();
        // All descendants sort after their parent; reverse byte order permits
        // bottom-up hashing without recursion or traversal-order dependence.
        for (path, kind) in self.entries.iter().rev() {
            let descendants: Vec<_> = children
                .get(path.as_str())
                .into_iter()
                .flatten()
                .map(|child| {
                    let e = &built[*child];
                    (&e.identity, &e.subtree_sha256)
                })
                .collect();
            let facts = match kind {
                EntryKind::File {
                    observation,
                    content_sha256,
                } => hash(&(
                    "file",
                    observation.bytes,
                    observation.executable,
                    &observation.classification_sha256,
                    content_sha256,
                ))?,
                _ => hash(kind)?,
            };
            let subtree_sha256 = hash(&(
                "rrflow-source-subtree-v1",
                identity(&self.scope, path)?,
                facts,
                descendants,
                path.is_empty().then_some((&self.revisions, &self.limits)),
            ))?;
            built.insert(
                path.clone(),
                Entry {
                    path: path.clone(),
                    identity: identity(&self.scope, path)?,
                    parent: if path.is_empty() {
                        None
                    } else {
                        Some(identity(&self.scope, parent(path))?)
                    },
                    kind: kind.clone(),
                    subtree_sha256,
                },
            );
        }
        let root_sha256 = built[""].subtree_sha256.clone();
        let entries: Vec<_> = built.into_values().collect();
        let changes = changes(
            self.prior.map(|p| p.entries.as_slice()).unwrap_or_default(),
            &entries,
        )?;
        let mut result = Proposal {
            format_version: FORMAT_VERSION,
            scope: self.scope,
            revisions: self.revisions,
            limits: self.limits,
            previous_sha256: self.prior.map(|p| p.proposal_sha256.clone()),
            complete: enumeration_complete
                && self.accounting.errors == 0
                && self.accounting.rejected_observations == 0,
            root_sha256,
            entries,
            changes,
            accounting: self.accounting,
            proposal_sha256: String::new(),
        };
        result.proposal_sha256 = hash(&("rrflow-source-proposal-v1", &result))?;
        Ok(result)
    }
}

fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 256
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}
fn digest(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(parent, _)| parent)
}
fn hash(value: &impl Serialize) -> Result<String> {
    serde_json::to_vec(value)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|_| InventoryError::Encoding)
}
fn identity(scope: &SourceScope, path: &str) -> Result<String> {
    hash(&("rrflow-source-entry-v1", scope, path))
}

fn changes(before: &[Entry], after: &[Entry]) -> Result<Changes> {
    let old: BTreeMap<_, _> = before.iter().map(|e| (e.path.as_str(), e)).collect();
    let new: BTreeMap<_, _> = after.iter().map(|e| (e.path.as_str(), e)).collect();
    let mut changes = Changes::default();
    for e in after {
        let change = EntryChange {
            path: e.path.clone(),
            identity: e.identity.clone(),
            before_sha256: old.get(e.path.as_str()).map(|p| p.subtree_sha256.clone()),
            after_sha256: Some(e.subtree_sha256.clone()),
        };
        match old.get(e.path.as_str()) {
            None => changes.added.push(change),
            Some(prior) if prior.subtree_sha256 != e.subtree_sha256 => {
                changes.modified.push(change)
            }
            _ => {}
        }
    }
    for e in before.iter().filter(|e| !new.contains_key(e.path.as_str())) {
        changes.removed.push(EntryChange {
            path: e.path.clone(),
            identity: e.identity.clone(),
            before_sha256: Some(e.subtree_sha256.clone()),
            after_sha256: None,
        });
    }
    let candidates = |entries: &[Entry],
                      other: &BTreeMap<&str, &Entry>|
     -> Result<BTreeMap<String, Vec<(String, String)>>> {
        let mut groups = BTreeMap::<String, Vec<(String, String)>>::new();
        for entry in entries
            .iter()
            .filter(|e| !other.contains_key(e.path.as_str()))
        {
            if let EntryKind::File {
                observation,
                content_sha256,
            } = &entry.kind
            {
                let signature = hash(&(
                    observation.bytes,
                    observation.executable,
                    &observation.classification_sha256,
                    content_sha256,
                ))?;
                groups
                    .entry(signature)
                    .or_default()
                    .push((entry.identity.clone(), content_sha256.clone()));
            }
        }
        Ok(groups)
    };
    let removed = candidates(before, &new)?;
    for (signature, additions) in candidates(after, &old)? {
        if let Some(removals) = removed.get(&signature) {
            if additions.len() == 1 && removals.len() == 1 {
                changes.rename_candidates.push(RenameCandidate {
                    removed_identity: removals[0].0.clone(),
                    added_identity: additions[0].0.clone(),
                    content_sha256: additions[0].1.clone(),
                });
            }
        }
    }
    Ok(changes)
}
