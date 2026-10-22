//! Hard bounds on active mutable-state accounting, distinct from soft flush targets.
use serde::{Deserialize, Serialize};

/// Key bytes (charged per version), value bytes and native version record bytes.
/// Excludes allocator/tree overhead, retained generations, transient input/decoding,
/// caches and maintenance. This is not an aggregate heap or RSS budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutableLimits {
    pub accounted_bytes: usize,
    pub versions: usize,
}

impl Default for MutableLimits {
    fn default() -> Self {
        Self {
            accounted_bytes: 128 * 1024 * 1024,
            versions: 1_048_576,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutableResource {
    AccountedBytes,
    Versions,
}

impl MutableResource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AccountedBytes => "accounted bytes",
            Self::Versions => "versions",
        }
    }
}

impl MutableLimits {
    pub(crate) fn validate(self) -> crate::Result<Self> {
        if self.accounted_bytes == 0 || self.versions == 0 {
            return Err(crate::Error::InvalidConfiguration(
                "hard mutable accounted-byte and version limits must be greater than zero".into(),
            ));
        }
        Ok(self)
    }

    pub(crate) fn check(self, charge: MutableCharge) -> crate::Result<()> {
        for (resource, limit, observed) in [
            (MutableResource::Versions, self.versions, charge.versions),
            (
                MutableResource::AccountedBytes,
                self.accounted_bytes,
                charge.bytes,
            ),
        ] {
            if observed > limit {
                return Err(crate::Error::MutableLimit {
                    resource,
                    limit,
                    observed,
                });
            }
        }
        Ok(())
    }

    // Subtraction avoids overflow even when a caller selects usize::MAX.
    pub(crate) fn needs_flush(self, current: MutableCharge, incoming: MutableCharge) -> bool {
        current.bytes > self.accounted_bytes.saturating_sub(incoming.bytes)
            || current.versions > self.versions.saturating_sub(incoming.versions)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MutableCharge {
    pub bytes: usize,
    pub versions: usize,
}

impl MutableCharge {
    pub(crate) fn batch(batch: &crate::WriteBatch) -> crate::Result<Self> {
        let mut bytes = 0usize;
        for operation in &batch.operations {
            let value_len = match operation {
                crate::Mutation::Put { value, .. } => value.len(),
                crate::Mutation::Delete { .. } => 0,
            };
            bytes = bytes
                .checked_add(version_charge(operation.key().len(), value_len)?)
                .ok_or_else(|| {
                    crate::Error::InvalidBatch("mutable byte accounting overflow".into())
                })?;
        }
        Ok(Self {
            bytes,
            versions: batch.len(),
        })
    }

    pub(crate) fn checked_add(self, other: Self) -> crate::Result<Self> {
        Ok(Self {
            bytes: self.bytes.checked_add(other.bytes).ok_or_else(|| {
                crate::Error::InvalidBatch("mutable byte accounting overflow".into())
            })?,
            versions: self.versions.checked_add(other.versions).ok_or_else(|| {
                crate::Error::InvalidBatch("mutable version accounting overflow".into())
            })?,
        })
    }
}

pub(crate) fn version_charge(key_len: usize, value_len: usize) -> crate::Result<usize> {
    key_len
        .checked_add(value_len)
        .and_then(|bytes| bytes.checked_add(std::mem::size_of::<crate::VersionedValue>()))
        .ok_or_else(|| {
            crate::Error::InvalidBatch("mutable version byte accounting overflow".into())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_never_wraps_a_large_charge_into_admission() {
        assert!(version_charge(usize::MAX, 1).is_err());
        assert!(MutableCharge {
            bytes: usize::MAX,
            versions: 1
        }
        .checked_add(MutableCharge {
            bytes: 1,
            versions: 1
        })
        .is_err());
        assert!(MutableCharge {
            bytes: 1,
            versions: usize::MAX
        }
        .checked_add(MutableCharge {
            bytes: 1,
            versions: 1
        })
        .is_err());
        let limits = MutableLimits {
            accounted_bytes: usize::MAX,
            versions: usize::MAX,
        };
        assert!(limits.needs_flush(
            MutableCharge {
                bytes: usize::MAX,
                versions: 0
            },
            MutableCharge {
                bytes: 1,
                versions: 1
            },
        ));
        assert!(limits.needs_flush(
            MutableCharge {
                bytes: 0,
                versions: usize::MAX
            },
            MutableCharge {
                bytes: 1,
                versions: 1
            },
        ));
        assert!(!limits.needs_flush(
            MutableCharge {
                bytes: usize::MAX - 1,
                versions: usize::MAX - 1
            },
            MutableCharge {
                bytes: 1,
                versions: 1
            },
        ));
    }
}
