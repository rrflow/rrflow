//! Runtime identifiers and canonical record references.

use super::*;

// A scope is family-qualified, as in `instance:{id}` and `maintenance:{id}`, and the public contract
// carries it as text.
text_ident!(ScopeId, "runtime scope", check_text);
text_ident!(RuntimeType, "runtime type", check_canonical);
text_ident!(@type RuntimeId, "runtime id");
text_ident!(SnapshotId, "snapshot id", check_canonical);
text_ident!(RetentionPinId, "retention pin id", check_canonical);
text_ident!(ProjectionId, "projection id", check_canonical);
text_ident!(OutboxId, "outbox id", check_canonical);

/// Wire version for the data-runtime transaction, snapshot, projection, and
/// audit contracts. Unknown versions are rejected rather than guessed.
pub const DATA_RUNTIME_CONTRACT_VERSION: u16 = 1;

/// A stable, typed reference within one runtime scope.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RuntimeRef {
    pub kind: RuntimeType,
    pub id: RuntimeId,
}

impl RuntimeRef {
    /// A reference by canonical id, which is all anything outside the kernel can name.
    pub fn new(kind: impl Into<String>, id: impl Into<String>) -> Result<Self> {
        Ok(Self {
            kind: RuntimeType::new(kind)?,
            id: RuntimeId::new(id)?,
        })
    }

    /// The event of this kind at `cursor`.
    pub fn event(kind: RuntimeType, cursor: u64) -> Self {
        Self {
            kind,
            id: RuntimeId::event(cursor),
        }
    }

    /// Reads a reference back from storage or a query, where it may name an event.
    pub fn parse(kind: impl Into<String>, id: impl Into<String>) -> Result<Self> {
        Ok(Self {
            kind: RuntimeType::new(kind)?,
            id: RuntimeId::parse(id)?,
        })
    }
}

pub(super) const EVENT_ID_PREFIX: &str = "cursor:";

impl RuntimeId {
    /// A canonical id. A caller cannot name an event this way.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        check_canonical("runtime id", &value)?;
        Ok(Self(value))
    }

    /// The identity of the event its commit placed at `cursor`. Its colon is outside the canonical
    /// grammar on purpose: it keeps an event apart from every id a caller can supply, so no record
    /// can collide with one.
    pub fn event(cursor: u64) -> Self {
        Self(format!("{EVENT_ID_PREFIX}{cursor}"))
    }

    /// The cursor this identity names, when it is an event's.
    pub fn event_cursor(&self) -> Option<u64> {
        self.0.strip_prefix(EVENT_ID_PREFIX)?.parse().ok()
    }

    /// Reads either form back, taking an event identity only as `event` writes it.
    pub fn parse(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if let Some(digits) = value.strip_prefix(EVENT_ID_PREFIX) {
            if digits
                .parse::<u64>()
                .is_ok_and(|cursor| cursor.to_string() == digits)
            {
                return Ok(Self(value));
            }
        }
        Self::new(value)
    }
}

impl TryFrom<String> for RuntimeId {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        Self::parse(value)
    }
}
