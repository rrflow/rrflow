use std::io;
use std::path::Path;

use rrd_contract::CanonicalId;
use serde::{Deserialize, Serialize};

mod private_file;
pub(in crate::engine) use private_file::{read_private_bounded, write_private_new};

pub const TOKEN_KEY_BYTES: usize = 32;
const API_KEY_DOCUMENT_FORMAT: u16 = 1;

/// Owner-only credential file bound to the exact accepted installation plan.
/// Debug output is deliberately redacted so adapters cannot accidentally log
/// the credential while reporting lifecycle errors.
#[derive(Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApiKeyDocument {
    pub format_version: u16,
    pub plan_sha256: String,
    pub principal_id: CanonicalId,
    pub credential_bytes: u16,
    credential: String,
}

#[derive(Serialize)]
struct ApiKeyDocumentWire<'a> {
    format_version: u16,
    plan_sha256: &'a str,
    principal_id: &'a CanonicalId,
    credential_bytes: u16,
    credential: &'a str,
}

impl std::fmt::Debug for ApiKeyDocument {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ApiKeyDocument")
            .field("format_version", &self.format_version)
            .field("plan_sha256", &self.plan_sha256)
            .field("principal_id", &self.principal_id)
            .field("credential_bytes", &self.credential_bytes)
            .field("credential", &"[REDACTED]")
            .finish()
    }
}

impl ApiKeyDocument {
    pub fn credential(&self) -> &str {
        &self.credential
    }

    fn validate(&self) -> io::Result<()> {
        if self.format_version != API_KEY_DOCUMENT_FORMAT
            || !is_sha256(&self.plan_sha256)
            || !(32..=4_096).contains(&self.credential_bytes)
            || self.credential.len() != usize::from(self.credential_bytes) * 2
            || !self
                .credential
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "API key document is malformed or does not meet its entropy policy",
            ));
        }
        Ok(())
    }
}

fn create_token_key(path: &Path) -> io::Result<[u8; TOKEN_KEY_BYTES]> {
    let key = generate_token_key()?;
    write_private_new(path, &key)?;
    Ok(key)
}

pub(in crate::engine) fn generate_token_key() -> io::Result<[u8; TOKEN_KEY_BYTES]> {
    let mut key = [0_u8; TOKEN_KEY_BYTES];
    getrandom::fill(&mut key)
        .map_err(|error| io::Error::other(format!("token-key entropy failed: {error}")))?;
    Ok(key)
}

pub(in crate::engine) fn generate_api_key_document(
    plan_sha256: &str,
    principal_id: CanonicalId,
    credential_bytes: u16,
) -> io::Result<ApiKeyDocument> {
    let mut entropy = vec![0_u8; usize::from(credential_bytes)];
    if !(32..=4_096).contains(&credential_bytes) || !is_sha256(plan_sha256) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "credential policy or installation plan digest is invalid",
        ));
    }
    getrandom::fill(&mut entropy)
        .map_err(|error| io::Error::other(format!("API-key entropy failed: {error}")))?;
    let mut credential = String::with_capacity(entropy.len() * 2);
    for byte in entropy {
        use std::fmt::Write as _;
        write!(&mut credential, "{byte:02x}").expect("writing to String cannot fail");
    }
    let document = ApiKeyDocument {
        format_version: API_KEY_DOCUMENT_FORMAT,
        plan_sha256: plan_sha256.into(),
        principal_id,
        credential_bytes,
        credential,
    };
    document.validate()?;
    Ok(document)
}

pub(in crate::engine) fn encode_api_key_document(document: &ApiKeyDocument) -> io::Result<Vec<u8>> {
    document.validate()?;
    serde_json::to_vec(&ApiKeyDocumentWire {
        format_version: document.format_version,
        plan_sha256: &document.plan_sha256,
        principal_id: &document.principal_id,
        credential_bytes: document.credential_bytes,
        credential: document.credential(),
    })
    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

pub(in crate::engine) fn decode_api_key_document(bytes: &[u8]) -> io::Result<ApiKeyDocument> {
    let document: ApiKeyDocument = serde_json::from_slice(bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    document.validate()?;
    Ok(document)
}

pub fn read_api_key_document(path: &Path) -> io::Result<ApiKeyDocument> {
    let bytes = read_private_bounded(path, 16 * 1024)?;
    decode_api_key_document(&bytes)
}

/// Loads the RRD token-signing key or creates it exactly once for retained
/// engine-internal component/control paths. Product transports and adapters
/// instead open the already-installed key through the installation authority.
pub(super) fn load_or_create_token_key(path: &Path) -> io::Result<[u8; TOKEN_KEY_BYTES]> {
    match read_token_key(path) {
        Ok(key) => return Ok(key),
        Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
        Err(_) => {}
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    match create_token_key(path) {
        Ok(key) => Ok(key),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => read_token_key(path),
        Err(error) => Err(error),
    }
}

pub fn read_token_key(path: &Path) -> io::Result<[u8; TOKEN_KEY_BYTES]> {
    let bytes = read_private_bounded(path, TOKEN_KEY_BYTES as u64)?;
    bytes.try_into().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "token key must contain exactly 32 bytes",
        )
    })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_reopen_preserve_one_key() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("RRD.SECRET");
        let created = load_or_create_token_key(&path).unwrap();
        let reopened = load_or_create_token_key(&path).unwrap();
        assert_eq!(created, reopened);
        assert_ne!(created, [0_u8; TOKEN_KEY_BYTES]);
    }

    #[test]
    fn secret_hardlinks_are_rejected_without_changing_either_path() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("RRD.SECRET");
        let alias = root.path().join("foreign-alias");
        let created = load_or_create_token_key(&path).unwrap();
        std::fs::hard_link(&path, &alias).unwrap();
        assert!(
            read_token_key(&path).is_err(),
            "multiply linked secrets must be denied"
        );
        assert!(
            read_token_key(&alias).is_err(),
            "the alias must also be denied"
        );
        assert_eq!(std::fs::read(&path).unwrap(), created);
        assert_eq!(std::fs::read(&alias).unwrap(), created);
        std::fs::remove_file(&alias).unwrap();
        assert_eq!(read_token_key(&path).unwrap(), created);
    }
}
