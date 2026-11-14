//! Security limits, errors, principals, grants, data policy, roles, JWT issuers and bindings, and authorization.

use super::*;

pub const SECURITY_FORMAT: u16 = 1;
pub const MAX_PRINCIPALS: usize = 4_096;
pub const MAX_ROLES: usize = 1_024;
pub const MAX_GRANTS_PER_PRINCIPAL: usize = 256;
pub const MAX_ROLE_DEPTH: usize = 16;
pub const MAX_IDENTITY_BINDINGS: usize = 4_096;
pub const MAX_JWT_ISSUERS: usize = 32;
pub const MAX_JWT_BYTES: usize = 16 * 1024;
pub const MAX_JWT_LIFETIME_MS: u64 = 60 * 60 * 1_000;
pub const MAX_AUDIT_PAGE: usize = 1_024;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Store(rrd_store::Error),
    Invalid(String),
    AlreadyInitialized,
    NotInitialized,
    PrincipalNotFound,
    Unauthenticated,
    PermissionDenied,
    IdempotencyConflict,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => write!(formatter, "security storage failed: {error}"),
            Self::Invalid(message) => write!(formatter, "invalid security state: {message}"),
            Self::AlreadyInitialized => formatter.write_str("security state already initialized"),
            Self::NotInitialized => formatter.write_str("security state is not initialized"),
            Self::PrincipalNotFound => formatter.write_str("security principal not found"),
            Self::Unauthenticated => formatter.write_str("principal credential was rejected"),
            Self::PermissionDenied => formatter.write_str("policy denied the requested action"),
            Self::IdempotencyConflict => formatter.write_str("security identity was rebound"),
        }
    }
}

impl std::error::Error for Error {}

impl From<rrd_store::Error> for Error {
    fn from(value: rrd_store::Error) -> Self {
        Self::Store(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalKind {
    User,
    Service,
    Node,
}

pub use rrd_contract::SecurityAction as Action;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceGrant {
    pub action: Action,
    /// Exact canonical resource prefix. An empty path grants no resource and is
    /// rejected; wildcard strings are never interpreted.
    pub resource_prefix: ResourcePath,
    /// Optional data-plane restriction compiled into query predicates and
    /// projection before physical planning. A caller that cannot enforce this
    /// context must deny the operation rather than treating it as a broad
    /// resource grant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_policy: Option<DataPolicy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyPredicate {
    pub field: String,
    pub value: RuntimeValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DataPolicy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tenant: Option<PolicyPredicate>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rows: Vec<PolicyPredicate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_fields: Option<BTreeSet<String>>,
}

impl DataPolicy {
    pub fn validate(&self) -> Result<()> {
        let mut fields = BTreeSet::new();
        for predicate in self.tenant.iter().chain(&self.rows) {
            validate_policy_field(&predicate.field)?;
            validate_policy_value(&predicate.value)?;
            if !fields.insert(predicate.field.as_str()) {
                return Err(Error::Invalid(
                    "data policy predicate fields must be unique".into(),
                ));
            }
        }
        if let Some(allowed) = &self.allowed_fields {
            if allowed.is_empty() || allowed.len() > 256 {
                return Err(Error::Invalid(
                    "field policy must contain 1..=256 fields".into(),
                ));
            }
            for field in allowed {
                validate_policy_field(field)?;
            }
        }
        if self.tenant.is_none() && self.rows.is_empty() && self.allowed_fields.is_none() {
            return Err(Error::Invalid("empty data policy is not permitted".into()));
        }
        Ok(())
    }

    pub fn predicates(&self) -> impl Iterator<Item = &PolicyPredicate> {
        self.tenant.iter().chain(&self.rows)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Role {
    pub id: CanonicalId,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub inherits: BTreeSet<CanonicalId>,
    pub grants: Vec<ResourceGrant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JwtIssuer {
    pub id: CanonicalId,
    pub issuer: String,
    pub audience: String,
    pub key_id: CanonicalId,
    pub signing_key_sha256: String,
    pub not_before_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityBinding {
    pub id: CanonicalId,
    pub issuer: String,
    pub subject: String,
    pub audience: String,
    pub principal_id: CanonicalId,
    pub disabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Principal {
    pub id: CanonicalId,
    pub kind: PrincipalKind,
    pub credential_sha256: String,
    #[serde(default = "initial_credential_revision")]
    pub credential_revision: u64,
    pub not_before_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub disabled: bool,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub role_ids: BTreeSet<CanonicalId>,
    pub grants: Vec<ResourceGrant>,
}

impl Principal {
    pub fn validate(&self) -> Result<()> {
        validate_sha256(&self.credential_sha256)?;
        if self.credential_revision == 0
            || self.not_before_unix_ms == 0
            || self.not_before_unix_ms >= self.expires_at_unix_ms
            || (self.grants.is_empty() && self.role_ids.is_empty())
            || self.grants.len() > MAX_GRANTS_PER_PRINCIPAL
        {
            return Err(Error::Invalid(
                "principal validity or grant bounds are invalid".into(),
            ));
        }
        let mut identities = BTreeSet::new();
        for grant in &self.grants {
            validate_grant(grant)?;
            let identity =
                serde_json::to_vec(grant).map_err(|error| Error::Invalid(error.to_string()))?;
            if !identities.insert(identity) {
                return Err(Error::Invalid("principal grants must be unique".into()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityState {
    pub format_version: u16,
    pub revision: u64,
    pub principals: BTreeMap<CanonicalId, Principal>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub roles: BTreeMap<CanonicalId, Role>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub identity_bindings: BTreeMap<CanonicalId, IdentityBinding>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub jwt_issuers: BTreeMap<CanonicalId, JwtIssuer>,
}

impl SecurityState {
    pub fn validate(&self) -> Result<()> {
        if self.format_version != SECURITY_FORMAT
            || self.revision == 0
            || self.principals.is_empty()
            || self.principals.len() > MAX_PRINCIPALS
            || self.roles.len() > MAX_ROLES
            || self.identity_bindings.len() > MAX_IDENTITY_BINDINGS
            || self.jwt_issuers.len() > MAX_JWT_ISSUERS
        {
            return Err(Error::Invalid(
                "unsupported, empty, or oversized security state".into(),
            ));
        }
        for (id, principal) in &self.principals {
            principal.validate()?;
            if id != &principal.id {
                return Err(Error::Invalid("principal map identity differs".into()));
            }
            if principal
                .role_ids
                .iter()
                .any(|role| !self.roles.contains_key(role))
            {
                return Err(Error::Invalid(
                    "principal references an unknown role".into(),
                ));
            }
        }
        validate_roles(self)?;
        validate_identity_bindings(self)?;
        validate_jwt_issuers(self)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authorization {
    pub principal_id: CanonicalId,
    pub principal_kind: PrincipalKind,
    pub action: Action,
    pub resource: ResourcePath,
    pub policy_revision: u64,
    pub credential_revision: u64,
    pub data_policy: Option<DataPolicy>,
    pub authorization_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JwtCredential {
    pub token: String,
    pub expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JwtIssueRequest {
    pub issuer_id: CanonicalId,
    pub token_id: CanonicalId,
    pub issued_at_unix_ms: u64,
    pub not_before_unix_ms: u64,
    pub expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JwtHeader {
    pub(super) alg: String,
    pub(super) typ: String,
    pub(super) kid: CanonicalId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JwtClaims {
    pub(super) iss: String,
    pub(super) sub: CanonicalId,
    pub(super) aud: String,
    pub(super) iat: u64,
    pub(super) nbf: u64,
    pub(super) exp: u64,
    pub(super) jti: CanonicalId,
    pub(super) policy_revision: u64,
    pub(super) credential_revision: u64,
}
