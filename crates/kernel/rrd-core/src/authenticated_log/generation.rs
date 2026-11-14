//! Source-generation selection, history, proof budgets and verification.

use super::*;

pub const RUNTIME_LOG_ACCUMULATOR_VERSION: u16 = 1;

/// Caller-admitted limits for source-generation proof processing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceGenerationProofBudget {
    pub max_identity_bytes: usize,
    pub max_path_nodes: usize,
}

impl SourceGenerationProofBudget {
    fn validate(self) -> Result<()> {
        // A u64-sized RFC 9162 tree has at most 64 proof levels.
        if self.max_identity_bytes == 0 || self.max_path_nodes == 0 || self.max_path_nodes > 64 {
            return invalid("invalid source generation proof budget");
        }
        Ok(())
    }
}

/// One completed source generation in an authority-ordered history. The
/// generation digest names its committed manifest; it is not itself proof
/// that any source entry belongs to that manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceGenerationSelection {
    pub scope: ScopeId,
    pub project: RuntimeId,
    pub completed_cursor: u64,
    pub generation_sha256: String,
}

impl SourceGenerationSelection {
    fn authenticated_bytes(&self, budget: SourceGenerationProofBudget) -> Result<Vec<u8>> {
        budget.validate()?;
        if self.completed_cursor == 0 {
            return invalid("a source generation must have a nonzero completion cursor");
        }
        // Identifiers in the broader runtime have no fixed byte bound. Bound
        // this proof input explicitly before allocating its canonical encoding.
        if self.scope.as_str().len() > budget.max_identity_bytes
            || self.project.as_str().len() > budget.max_identity_bytes
        {
            return invalid("source generation scope or project exceeds its proof bound");
        }
        parse_digest("source generation digest", &self.generation_sha256)?;
        let mut bytes = b"rrflow-source-generation-selection-v1\0".to_vec();
        for value in [self.scope.as_str(), self.project.as_str()] {
            bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
            bytes.extend_from_slice(value.as_bytes());
        }
        bytes.extend_from_slice(&self.completed_cursor.to_be_bytes());
        bytes.extend_from_slice(self.generation_sha256.as_bytes());
        Ok(bytes)
    }
}

/// Ephemeral construction of one ordered history. Future storage integration
/// must make ordering and its root durable atomically with publication; this
/// in-memory builder is not a store, an authorization boundary, or a reader.
pub struct SourceGenerationHistory {
    scope: ScopeId,
    project: RuntimeId,
    last_cursor: Option<u64>,
    accumulator: RuntimeLogAccumulator,
    budget: SourceGenerationProofBudget,
}

impl SourceGenerationHistory {
    pub fn new(
        scope: ScopeId,
        project: RuntimeId,
        budget: SourceGenerationProofBudget,
    ) -> Result<Self> {
        budget.validate()?;
        if scope.as_str().len() > budget.max_identity_bytes
            || project.as_str().len() > budget.max_identity_bytes
        {
            return invalid("source generation scope or project exceeds its proof bound");
        }
        Ok(Self {
            scope,
            project,
            last_cursor: None,
            accumulator: RuntimeLogAccumulator::new(),
            budget,
        })
    }

    /// Restores the ordered writer from an externally authenticated accumulator
    /// and its last leaf. The caller must authenticate the root at the correct
    /// runtime stamp and establish that all earlier appends were ordered; this
    /// check alone cannot prove either fact or the integrity of stored nodes.
    pub fn from_trusted_accumulator(
        scope: ScopeId,
        project: RuntimeId,
        accumulator: RuntimeLogAccumulator,
        last: Option<&SourceGenerationNeighbor>,
        budget: SourceGenerationProofBudget,
    ) -> Result<Self> {
        budget.validate()?;
        if scope.as_str().len() > budget.max_identity_bytes
            || project.as_str().len() > budget.max_identity_bytes
        {
            return invalid("source generation scope or project exceeds its proof bound");
        }
        accumulator.validate()?;
        let last_cursor = match (accumulator.tree_size, last) {
            (0, None) => None,
            (size, Some(neighbor)) if size > 0 => {
                if neighbor.selection.scope != scope
                    || neighbor.selection.project != project
                    || neighbor.inclusion.tree_size != size
                    || neighbor.inclusion.leaf_index != size - 1
                    || neighbor.inclusion.path.len() > budget.max_path_nodes
                {
                    return invalid("source generation history tail identity mismatch");
                }
                neighbor.inclusion.verify(
                    &neighbor.selection.authenticated_bytes(budget)?,
                    &accumulator.root,
                )?;
                Some(neighbor.selection.completed_cursor)
            }
            _ => return invalid("source generation history tail is missing or unexpected"),
        };
        Ok(Self {
            scope,
            project,
            last_cursor,
            accumulator,
            budget,
        })
    }

    pub fn append(
        &mut self,
        selection: &SourceGenerationSelection,
    ) -> Result<Vec<RuntimeMerkleNode>> {
        if selection.scope != self.scope || selection.project != self.project {
            return invalid("source generation history scope or project mismatch");
        }
        if self
            .last_cursor
            .is_some_and(|last| selection.completed_cursor <= last)
        {
            return invalid("source generation completion cursors must strictly increase");
        }
        let bytes = selection.authenticated_bytes(self.budget)?;
        let nodes = self.accumulator.append(&bytes)?;
        self.last_cursor = Some(selection.completed_cursor);
        Ok(nodes)
    }

    pub fn accumulator(&self) -> &RuntimeLogAccumulator {
        &self.accumulator
    }
}

/// An authenticated neighbor of a requested runtime cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceGenerationNeighbor {
    pub selection: SourceGenerationSelection,
    pub inclusion: RuntimeInclusionProof,
}

/// Verifies a predecessor/successor pair in a strictly ordered selection log.
///
/// This only proves the answer relative to `trusted_root` and the ordering
/// invariant enforced by the writer. The caller MUST authenticate that root
/// and its tree size for the requested runtime stamp, and must establish the
/// writer's ordering invariant. An inclusion proof or an untrusted root alone
/// does not establish freshness. No current engine path is wired to this yet.
#[allow(clippy::too_many_arguments)]
pub fn verify_source_generation_at<'a>(
    scope: &ScopeId,
    project: &RuntimeId,
    cursor: u64,
    trusted_root: &str,
    tree_size: u64,
    predecessor: Option<&'a SourceGenerationNeighbor>,
    successor: Option<&SourceGenerationNeighbor>,
    budget: SourceGenerationProofBudget,
) -> Result<Option<&'a SourceGenerationSelection>> {
    budget.validate()?;
    if scope.as_str().len() > budget.max_identity_bytes
        || project.as_str().len() > budget.max_identity_bytes
    {
        return invalid("source generation scope or project exceeds its proof bound");
    }
    if tree_size == 0 {
        if predecessor.is_some()
            || successor.is_some()
            || trusted_root != RuntimeLogAccumulator::new().root
        {
            return invalid("invalid empty source generation history proof");
        }
        return Ok(None);
    }
    let verify = |neighbor: &SourceGenerationNeighbor| -> Result<()> {
        if &neighbor.selection.scope != scope
            || &neighbor.selection.project != project
            || neighbor.inclusion.tree_size != tree_size
            || neighbor.inclusion.path.len() > budget.max_path_nodes
        {
            return invalid("source generation proof identity or bound mismatch");
        }
        neighbor.inclusion.verify(
            &neighbor.selection.authenticated_bytes(budget)?,
            trusted_root,
        )
    };
    if let Some(left) = predecessor {
        verify(left)?;
        if left.selection.completed_cursor > cursor {
            return invalid("source generation predecessor is newer than the requested cursor");
        }
    }
    if let Some(right) = successor {
        verify(right)?;
        if right.selection.completed_cursor <= cursor {
            return invalid("source generation successor is not newer than the requested cursor");
        }
    }
    match (predecessor, successor) {
        (None, Some(right)) if right.inclusion.leaf_index == 0 => Ok(None),
        (Some(left), None) if left.inclusion.leaf_index.checked_add(1) == Some(tree_size) => {
            Ok(Some(&left.selection))
        }
        (Some(left), Some(right))
            if left.inclusion.leaf_index.checked_add(1) == Some(right.inclusion.leaf_index)
                && left.selection.completed_cursor < right.selection.completed_cursor =>
        {
            Ok(Some(&left.selection))
        }
        _ => invalid("source generation proof omits or misorders a neighbor"),
    }
}
