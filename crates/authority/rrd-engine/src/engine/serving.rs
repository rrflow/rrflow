//! The served endpoint record: where a serving installed engram listens.
//!
//! `rrflow serve` publishes it once its listener is bound, so another local
//! process can find an instance started from the project folder. It is an
//! endpoint candidate, never readiness: a caller proves it with the
//! authenticated readiness challenge.

use super::engram_layout::{safe_join, EngramFilesystem, ENGRAM_DIRECTORY};
use super::installation::{
    publish_new_file, read_locator, read_locator_anchored, read_regular_bounded,
};
use super::*;
use rrd_store::filesystem::AnchoredPath;
use std::net::SocketAddr;

pub(super) const SERVING_RECORD_RELATIVE_PATH: &str = ".rrflow/serving.json";
const SERVING_RECORD_FORMAT: u16 = 1;
const MAX_SERVING_RECORD_BYTES: u64 = 4 * 1024;
const MAX_PRODUCT_VERSION_BYTES: usize = 64;

/// Where one serving process listens. It holds no credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServingRecord {
    pub format_version: u16,
    pub instance_id: CanonicalId,
    pub url: String,
    pub process_id: u32,
    pub product_version: String,
}

impl ServingRecord {
    /// The loopback address the record names. A readiness proof sends the
    /// installed credential to it, so any other address is refused.
    pub fn address(&self) -> Result<SocketAddr> {
        let address: SocketAddr = self
            .url
            .strip_prefix("http://")
            .and_then(|authority| authority.parse().ok())
            .ok_or_else(|| record_error("does not name an http://host:port URL"))?;
        if !address.ip().is_loopback() {
            return Err(record_error("names a non-loopback address"));
        }
        if format!("http://{address}") != self.url {
            return Err(record_error("does not hold a canonical URL"));
        }
        Ok(address)
    }

    fn validate(&self, instance_id: &CanonicalId) -> Result<()> {
        if self.format_version != SERVING_RECORD_FORMAT {
            return Err(record_error("has an unsupported format version"));
        }
        if &self.instance_id != instance_id {
            return Err(record_error("names another instance"));
        }
        if self.product_version.is_empty() || self.product_version.len() > MAX_PRODUCT_VERSION_BYTES
        {
            return Err(record_error("has an invalid product version"));
        }
        self.address().map(|_| ())
    }
}

/// Publishes this process's endpoint for an open installed engram. Only an
/// open installed engine issues one, and that engine holds the engram's single
/// writer lock, so a record found at publication was left by a process that no
/// longer runs.
#[derive(Debug)]
pub struct ServingPublisher {
    project: EngramFilesystem,
    instance_id: CanonicalId,
    product_version: String,
}

/// A published record. Withdraw it on clean shutdown.
#[derive(Debug)]
pub struct ServingRegistration {
    path: AnchoredPath,
    bytes: Vec<u8>,
    record: ServingRecord,
}

impl RrdEngine {
    pub fn serving_publisher(&self) -> Result<ServingPublisher> {
        let root = self.installed_project_root.clone().ok_or_else(|| {
            ServiceError::Contract("only an installed engram records a served endpoint".into())
        })?;
        let (project, locator, _) = read_locator_anchored(&EngramFilesystem::admit(root, false)?)?;
        if &locator.identity.instance_id != self.instance_id() {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        Ok(ServingPublisher {
            project,
            instance_id: locator.identity.instance_id,
            product_version: locator.product_version,
        })
    }

    /// Reads an installed project's served endpoint record, if one exists.
    /// Only the authenticated readiness challenge proves it.
    pub fn read_serving_record(project: &Path) -> Result<Option<ServingRecord>> {
        let (project, locator, _) = read_locator(project)?;
        let path = safe_join(&project, SERVING_RECORD_RELATIVE_PATH)?;
        match path.metadata() {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(ServiceError::Storage(error.to_string())),
        }
        let bytes = read_record_file(&path)?;
        let record: ServingRecord = serde_json::from_slice(&bytes)
            .map_err(|error| record_error(&format!("is malformed: {error}")))?;
        record.validate(&locator.identity.instance_id)?;
        Ok(Some(record))
    }
}

impl ServingPublisher {
    pub fn publish(&self, address: SocketAddr) -> Result<ServingRegistration> {
        let record = ServingRecord {
            format_version: SERVING_RECORD_FORMAT,
            instance_id: self.instance_id.clone(),
            url: format!("http://{address}"),
            process_id: std::process::id(),
            product_version: self.product_version.clone(),
        };
        record.validate(&self.instance_id)?;
        let bytes = serde_json::to_vec(&record)
            .map_err(|error| ServiceError::Contract(error.to_string()))?;
        let path = safe_join(&self.project, SERVING_RECORD_RELATIVE_PATH)?;
        let temporary = safe_join(
            &self.project,
            &format!("{ENGRAM_DIRECTORY}/.serving.pending"),
        )?;
        remove_stale(&temporary)?;
        remove_stale(&path)?;
        publish_new_file(&path, &temporary, &bytes)?;
        Ok(ServingRegistration {
            path,
            bytes,
            record,
        })
    }
}

impl ServingRegistration {
    pub fn record(&self) -> &ServingRecord {
        &self.record
    }

    /// Removes the record if it is still the one this process published.
    pub fn withdraw(self) -> Result<()> {
        match self.path.metadata() {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(ServiceError::Storage(error.to_string())),
        }
        if read_record_file(&self.path)? != self.bytes {
            return Ok(());
        }
        remove_and_sync(&self.path)
    }
}

/// Removes a record left by a process that no longer holds the engram. Only a
/// bounded regular file is removed; anything else is refused, not replaced.
fn remove_stale(path: &AnchoredPath) -> Result<()> {
    match path.metadata() {
        Ok(_) => {
            read_record_file(path)?;
            remove_and_sync(path)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(ServiceError::Storage(error.to_string())),
    }
}

fn read_record_file(path: &AnchoredPath) -> Result<Vec<u8>> {
    read_regular_bounded(path, MAX_SERVING_RECORD_BYTES).map_err(|error| match error {
        ServiceError::ProjectBindingMismatch => {
            record_error("is not a bounded regular file; remove it to serve this engram")
        }
        other => other,
    })
}

fn remove_and_sync(path: &AnchoredPath) -> Result<()> {
    path.remove_file()
        .map_err(|error| ServiceError::Storage(error.to_string()))?;
    path.parent()
        .ok_or(ServiceError::ProjectBindingMismatch)?
        .sync_directory()
        .map_err(|error| ServiceError::Storage(error.to_string()))
}

fn record_error(reason: &str) -> ServiceError {
    ServiceError::Contract(format!("the served endpoint record {reason}"))
}
