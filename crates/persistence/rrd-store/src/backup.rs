//! Digest-verified local backup catalogue over retained logical archives.
//!
//! Every digest here detects changed bytes. None authenticates a producer:
//! the catalogue shares the estate's filesystem trust domain, so a keyed
//! authenticator would be stored beside the key it protects. Operations that
//! read a catalogue say "verify", never "authenticate".

use crate::access::runtime_state::{checked_key, get};
use crate::archive::{ControlArchiveWriter, CATALOGUE_CONTROL_FAMILIES};
use crate::filesystem::{AnchorInput, AnchoredPath, OpenOptions};
use crate::keyspaces::{self, Durability};
use crate::repository::prepare_control_batch;
use crate::{
    control_family_covers, control_family_of, export_logical_archive, inspect_control_archive,
    inspect_logical_archive, read_control_archive,
    restore_logical_archive_to_new_root_with_options, validate_control_families,
    ControlArchiveInventory, ControlTransition, Error, ImmutableObjectStore, LocalObjectStore,
    LogicalArchiveInventory, LogicalRestoreReport, Result, RrflowKvStore, StorageEngine,
    CONTROL_ARCHIVE_VERSION,
};
use rrd_core::digest;
use rrd_core::{ObjectReference, RuntimeMutation, ScopeId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::{Component, Path};
use std::sync::atomic::{AtomicU64, Ordering};

/// Version 2 adds the control-journal archive: an application-complete backup
/// carries the estate's control state (installed record, configuration,
/// security policy and audit chain, attunement) as replayable journal entries.
/// A version 1 catalogue is refused rather than read as complete.
pub const BACKUP_CATALOGUE_VERSION: u16 = 2;
pub const CONTROL_RESTORE_PROGRESS_FORMAT: u16 = 1;
const CATALOGUE_FILE: &str = "catalogue.json";
const ARCHIVE_DIRECTORY: &str = "archives";
const OBJECT_MANIFEST_DIRECTORY: &str = "object-manifests";
const CATALOGUE_MANIFEST_DIRECTORY: &str = "catalogue-manifests";
const CONTROL_ARCHIVE_DIRECTORY: &str = "control-archives";
const OBJECT_PAYLOAD_DIRECTORY: &str = "payloads";
const OBJECT_MANIFEST_VERSION: u16 = 1;
const PAGE_SIZE: usize = 1_024;
const CATALOGUE_MANIFEST_VERSION: u16 = 1;
const MAX_ARCHIVE_OBJECTS: usize = 1_000_000;
const MAX_ARCHIVE_CATALOGUE_RECORDS: usize = 1_000_000;
const MAX_ARCHIVE_MANIFEST_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_BACKUP_PRUNE_RECEIPTS: usize = 4_096;
static BACKUP_TEMP_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupCoverage {
    Included,
    ReferencedOnly,
    RebuildRequired,
    Excluded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupEntry {
    pub backup_id: String,
    pub label: String,
    pub created_at: u64,
    pub archive_file: String,
    pub archive: LogicalArchiveInventory,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_manifest_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_manifest: Option<ObjectPayloadManifestInventory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalogue_manifest_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalogue_manifest: Option<CatalogueManifestInventory>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_archive_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_archive: Option<ControlArchiveInventory>,
    pub claims: BackupCoverage,
    pub typed_runtime: BackupCoverage,
    pub catalogues: BackupCoverage,
    /// The estate's control state outside the catalogues: installed record and
    /// checkpoint, active configuration, security policy and audit chain, and
    /// attunement plan and job state, carried as replayable journal entries.
    pub installed_control: BackupCoverage,
    pub object_payloads: BackupCoverage,
    pub projections: BackupCoverage,
    pub invocation_telemetry: BackupCoverage,
    pub snapshot_leases: BackupCoverage,
    pub application_complete: bool,
}

/// Durable progress of one control-journal archive replay into a restored
/// root. It is written in the same physical transaction as each replayed
/// transition and stays in the root as restore provenance once complete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlRestoreProgress {
    pub format_version: u16,
    pub archive_sha256: String,
    pub entry_count: u64,
    pub applied: u64,
    pub complete: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_restored_sequence: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_restored_sequence: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectPayloadManifestInventory {
    pub format_version: u16,
    pub manifest_sha256: String,
    pub object_count: u64,
    pub payload_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogueManifestInventory {
    pub format_version: u16,
    pub manifest_sha256: String,
    pub source_control_sequence: u64,
    pub scope_count: u64,
    pub record_count: u64,
    pub payload_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogueScopeSnapshot {
    scope: ScopeId,
    revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogueRecordSnapshot {
    scope: ScopeId,
    key: String,
    value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CatalogueManifest {
    format_version: u16,
    source_control_sequence: u64,
    scopes: Vec<CatalogueScopeSnapshot>,
    records: Vec<CatalogueRecordSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredCatalogueManifest {
    payload: CatalogueManifest,
    payload_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObjectPayloadEntry {
    sha256: String,
    length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObjectPayloadManifest {
    format_version: u16,
    objects: Vec<ObjectPayloadEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredObjectPayloadManifest {
    payload: ObjectPayloadManifest,
    payload_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupCatalogue {
    pub format_version: u16,
    pub revision: u64,
    pub catalogue_sha256: String,
    pub backups: Vec<BackupEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prune_receipts: Vec<BackupPruneReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupPrunePlan {
    pub expected_catalogue_sha256: String,
    pub retained_backup_ids: Vec<String>,
    pub prune_candidate_backup_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackupPruneReceipt {
    pub request_sha256: String,
    pub expected_catalogue_sha256: String,
    pub retained_backup_ids: Vec<String>,
    pub removed_backups: Vec<BackupEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupPruneOutcome {
    pub catalogue: BackupCatalogue,
    pub pruned_backup_ids: Vec<String>,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CataloguePayload {
    format_version: u16,
    revision: u64,
    backups: Vec<BackupEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    prune_receipts: Vec<BackupPruneReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredCatalogue {
    payload: CataloguePayload,
    payload_sha256: String,
}

/// Creates or reuses a content-addressed archive and atomically adds its
/// coverage record to the catalogue.
pub fn create_logical_backup<E: StorageEngine>(
    engine: &E,
    catalogue_root: &(impl AnchorInput + ?Sized),
    label: &str,
    created_at: u64,
) -> Result<BackupEntry> {
    let catalogue_root = &catalogue_root.anchor().map_err(backup_io)?;
    validate_label(label)?;
    initialize_catalogue(catalogue_root)?;
    let archives = catalogue_root.join(ARCHIVE_DIRECTORY);
    archives.create_dir_all().map_err(backup_io)?;
    let id = BACKUP_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let staging = archives.join(format!(".logical-backup-{}-{id}.tmp", std::process::id()));
    let inventory = export_logical_archive(engine, &staging)?;
    let archive_name = format!("{}.rrd-archive", inventory.archive_sha256);
    let archive_path = archives.join(&archive_name);
    if archive_path.try_exists().map_err(backup_io)? {
        let retained = inspect_logical_archive(&archive_path)?;
        if retained != inventory {
            let _ = staging.remove_file();
            return Err(Error::Archive(
                "content-addressed backup archive conflicts with retained inventory".into(),
            ));
        }
        staging.remove_file().map_err(backup_io)?;
    } else {
        staging.rename_to(&archive_path).map_err(backup_io)?;
    }

    let backup_id = digest::sha256_hex(
        format!(
            "rrd-logical-backup-v1\0{label}\0{created_at}\0{}",
            inventory.archive_sha256
        )
        .as_bytes(),
    );
    let entry = BackupEntry {
        backup_id,
        label: label.into(),
        created_at,
        archive_file: format!("{ARCHIVE_DIRECTORY}/{archive_name}"),
        archive: inventory,
        object_manifest_file: None,
        object_manifest: None,
        catalogue_manifest_file: None,
        catalogue_manifest: None,
        control_archive_file: None,
        control_archive: None,
        claims: BackupCoverage::Included,
        typed_runtime: BackupCoverage::Included,
        catalogues: BackupCoverage::Excluded,
        installed_control: BackupCoverage::Excluded,
        object_payloads: BackupCoverage::ReferencedOnly,
        projections: BackupCoverage::RebuildRequired,
        invocation_telemetry: BackupCoverage::Excluded,
        snapshot_leases: BackupCoverage::Excluded,
        application_complete: false,
    };

    let mut catalogue = load_backup_catalogue(catalogue_root)?;
    deny_pruned_identity_reuse(&catalogue, &entry.backup_id)?;
    if let Some(existing) = catalogue
        .backups
        .iter()
        .find(|existing| existing.backup_id == entry.backup_id)
    {
        if existing == &entry {
            return Ok(existing.clone());
        }
        return Err(Error::Archive("backup identity collision".into()));
    }
    catalogue.revision = catalogue
        .revision
        .checked_add(1)
        .ok_or_else(|| Error::Archive("backup catalogue revision overflow".into()))?;
    catalogue.backups.push(entry.clone());
    catalogue.backups.sort_by(|left, right| {
        (left.created_at, &left.backup_id).cmp(&(right.created_at, &right.backup_id))
    });
    write_catalogue(catalogue_root, catalogue)?;
    Ok(entry)
}

/// Creates an application-complete logical backup by retaining the exact
/// immutable-object closure referenced by the exported runtime cut, the
/// catalogue control state, and the control-journal entries of the declared
/// `control_families`.
///
/// The caller owns the control-key layout and declares which families are
/// estate control state. Secrets never live in control values (the token key
/// and operator credential are files outside every family), and transient
/// families such as sessions, subscriptions, and operation receipts stay out
/// of the scope. Logical replay, object bytes, catalogues, and control entries
/// remain separate physical artifacts, but one digest-verified catalogue entry
/// binds their content identities. All repositories read one admitted physical
/// snapshot, retained through publication; concurrent writes belong to a later
/// cut and cannot mix into this archive.
pub fn create_application_backup<E: StorageEngine, O: ImmutableObjectStore>(
    engine: &E,
    objects: &O,
    catalogue_root: &(impl AnchorInput + ?Sized),
    label: &str,
    created_at: u64,
    control_families: &[String],
) -> Result<BackupEntry> {
    let snapshot = crate::snapshot::SnapshotStore::capture(engine)?;
    let engine = &snapshot;
    let catalogue_root = &catalogue_root.anchor().map_err(backup_io)?;
    validate_label(label)?;
    validate_control_families(control_families)?;
    initialize_catalogue(catalogue_root)?;
    let control_sequence = engine.control().sequence()?;
    let archives = catalogue_root.join(ARCHIVE_DIRECTORY);
    archives.create_dir_all().map_err(backup_io)?;
    let id = BACKUP_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let staging = archives.join(format!(".logical-backup-{}-{id}.tmp", std::process::id()));
    let inventory = export_logical_archive(engine, &staging)?;
    let archive_name = format!("{}.rrd-archive", inventory.archive_sha256);
    let archive_path = archives.join(&archive_name);
    retain_content_addressed_file(&staging, &archive_path, &inventory)?;

    let references = object_references_at(engine, inventory.runtime_cursor)?;
    if engine.claims().sequence()? != inventory.claim_sequence
        || engine.runtime().cursor()? != inventory.runtime_cursor
    {
        return Err(Error::Archive(
            "source watermarks changed while retaining object payloads; retry from a stable cut"
                .into(),
        ));
    }
    let (manifest_file, manifest_inventory) =
        retain_object_payloads(objects, catalogue_root, &references)?;
    let (catalogue_manifest_file, catalogue_manifest) = retain_catalogues(engine, catalogue_root)?;
    let (control_archive_file, control_archive) =
        retain_control_journal(engine, catalogue_root, control_sequence, control_families)?;
    if engine.claims().sequence()? != inventory.claim_sequence
        || engine.runtime().cursor()? != inventory.runtime_cursor
        || engine.control().sequence()? != control_sequence
        || catalogue_manifest.source_control_sequence != control_sequence
        || control_archive.source_control_sequence != control_sequence
    {
        return Err(Error::Archive(
            "source data or control watermarks changed before application-complete backup publication; retry"
                .into(),
        ));
    }

    let backup_id = digest::sha256_hex(
        format!(
            "rrd-application-backup-v2\0{label}\0{created_at}\0{}\0{}\0{}\0{}",
            inventory.archive_sha256,
            manifest_inventory.manifest_sha256,
            catalogue_manifest.manifest_sha256,
            control_archive.archive_sha256
        )
        .as_bytes(),
    );
    let entry = BackupEntry {
        backup_id,
        label: label.into(),
        created_at,
        archive_file: format!("{ARCHIVE_DIRECTORY}/{archive_name}"),
        archive: inventory,
        object_manifest_file: Some(manifest_file),
        object_manifest: Some(manifest_inventory),
        catalogue_manifest_file: Some(catalogue_manifest_file),
        catalogue_manifest: Some(catalogue_manifest),
        control_archive_file: Some(control_archive_file),
        control_archive: Some(control_archive),
        claims: BackupCoverage::Included,
        typed_runtime: BackupCoverage::Included,
        catalogues: BackupCoverage::Included,
        installed_control: BackupCoverage::Included,
        object_payloads: BackupCoverage::Included,
        projections: BackupCoverage::RebuildRequired,
        invocation_telemetry: BackupCoverage::Excluded,
        snapshot_leases: BackupCoverage::Excluded,
        application_complete: true,
    };
    retain_catalogue_entry(catalogue_root, entry)
}

/// Loads the catalogue and verifies its payload digest. Archive bytes are
/// verified separately by `verify_backup_catalogue` so listing remains bounded.
///
/// A catalogue that was never written reads as empty. A catalogue file that
/// is absent while retained artifacts exist beside it fails closed: those
/// artifacts belonged to entries, and reading them as "no backups" would hide
/// the loss.
pub fn load_backup_catalogue(
    catalogue_root: &(impl AnchorInput + ?Sized),
) -> Result<BackupCatalogue> {
    let catalogue_root = &catalogue_root.anchor().map_err(backup_io)?;
    let path = catalogue_root.join(CATALOGUE_FILE);
    if !path.try_exists().map_err(backup_io)? {
        require_no_orphan_artifacts(catalogue_root)?;
        return catalogue_from_payload(empty_catalogue_payload());
    }
    let bytes = path.read().map_err(backup_io)?;
    let stored: StoredCatalogue = serde_json::from_slice(&bytes)?;
    if stored.payload.format_version != BACKUP_CATALOGUE_VERSION {
        return Err(Error::Archive(format!(
            "unsupported backup catalogue version {}",
            stored.payload.format_version
        )));
    }
    let actual = payload_digest(&stored.payload)?;
    if actual != stored.payload_sha256 {
        return Err(Error::Archive(
            "backup catalogue payload digest does not match".into(),
        ));
    }
    validate_entries(&stored.payload.backups)?;
    validate_prune_receipts(&stored.payload.prune_receipts)?;
    Ok(BackupCatalogue {
        format_version: stored.payload.format_version,
        revision: stored.payload.revision,
        catalogue_sha256: actual,
        backups: stored.payload.backups,
        prune_receipts: stored.payload.prune_receipts,
    })
}

/// Verifies the digests of the catalogue and of every retained artifact it
/// references, reading only: nothing is created or repaired. The digests
/// detect changed bytes; they do not authenticate a producer.
pub fn verify_backup_catalogue(
    catalogue_root: &(impl AnchorInput + ?Sized),
) -> Result<BackupCatalogue> {
    let catalogue_root = &catalogue_root.anchor().map_err(backup_io)?;
    let catalogue = load_backup_catalogue(catalogue_root)?;
    for entry in &catalogue.backups {
        let path = resolve_archive(catalogue_root, &entry.archive_file)?;
        let actual = inspect_logical_archive(&path)?;
        if actual != entry.archive {
            return Err(Error::Archive(format!(
                "backup {} archive inventory does not match catalogue",
                entry.backup_id
            )));
        }
        if entry.application_complete {
            load_catalogue_manifest_for_entry(catalogue_root, entry)?;
            load_control_archive_for_entry(catalogue_root, entry)?;
            verify_object_payloads(catalogue_root, entry)?;
        }
    }
    Ok(catalogue)
}

/// Publishes one digest-verified successor catalogue for a complete retention
/// partition, then reclaims only artifacts that no retained entry references.
/// Exact replay is recovered from the bounded receipt embedded in the
/// catalogue.
pub fn prune_backup_catalogue(
    catalogue_root: &(impl AnchorInput + ?Sized),
    plan: &BackupPrunePlan,
) -> Result<BackupPruneOutcome> {
    let catalogue_root = &catalogue_root.anchor().map_err(backup_io)?;
    validate_prune_plan(plan)?;
    let request_sha256 = digest::sha256_hex(&serde_json::to_vec(plan)?);
    let mut catalogue = verify_backup_catalogue(catalogue_root)?;
    if let Some(receipt) = catalogue
        .prune_receipts
        .iter()
        .find(|receipt| receipt.request_sha256 == request_sha256)
        .cloned()
    {
        cleanup_pruned_artifacts(catalogue_root, &receipt.removed_backups, &catalogue.backups)?;
        return Ok(BackupPruneOutcome {
            catalogue,
            pruned_backup_ids: sorted_backup_ids(
                receipt
                    .removed_backups
                    .iter()
                    .map(|entry| entry.backup_id.clone())
                    .collect(),
            ),
            idempotent_replay: true,
        });
    }
    if catalogue.catalogue_sha256 != plan.expected_catalogue_sha256 {
        return Err(Error::Archive(
            "backup prune expected catalogue digest is stale".into(),
        ));
    }
    if catalogue.prune_receipts.len() == MAX_BACKUP_PRUNE_RECEIPTS {
        return Err(Error::Archive(
            "backup prune receipt history is at its v1 bound".into(),
        ));
    }
    let current = catalogue
        .backups
        .iter()
        .map(|entry| entry.backup_id.clone())
        .collect::<BTreeSet<_>>();
    let retained = plan
        .retained_backup_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let candidates = plan
        .prune_candidate_backup_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if !retained.is_disjoint(&candidates)
        || retained
            .union(&candidates)
            .cloned()
            .collect::<BTreeSet<_>>()
            != current
    {
        return Err(Error::Archive(
            "backup prune plan is not a complete disjoint catalogue partition".into(),
        ));
    }
    let removed_backups = catalogue
        .backups
        .iter()
        .filter(|entry| candidates.contains(&entry.backup_id))
        .cloned()
        .collect::<Vec<_>>();
    catalogue
        .backups
        .retain(|entry| retained.contains(&entry.backup_id));
    catalogue.revision = catalogue
        .revision
        .checked_add(1)
        .ok_or_else(|| Error::Archive("backup catalogue revision overflow".into()))?;
    catalogue.prune_receipts.push(BackupPruneReceipt {
        request_sha256,
        expected_catalogue_sha256: plan.expected_catalogue_sha256.clone(),
        retained_backup_ids: plan.retained_backup_ids.clone(),
        removed_backups: removed_backups.clone(),
    });
    write_catalogue(catalogue_root, catalogue)?;
    let catalogue = verify_backup_catalogue(catalogue_root)?;
    cleanup_pruned_artifacts(catalogue_root, &removed_backups, &catalogue.backups)?;
    Ok(BackupPruneOutcome {
        catalogue,
        pruned_backup_ids: plan.prune_candidate_backup_ids.clone(),
        idempotent_replay: false,
    })
}

pub fn restore_catalogued_backup(
    catalogue_root: &(impl AnchorInput + ?Sized),
    backup_id: &str,
    target: &(impl AnchorInput + ?Sized),
    at: u64,
) -> Result<LogicalRestoreReport> {
    restore_catalogued_backup_with_options(
        catalogue_root,
        backup_id,
        target,
        at,
        Default::default(),
    )
}

/// Restores using the same native policy for private staging and every reopen.
pub fn restore_catalogued_backup_with_options(
    catalogue_root: &(impl AnchorInput + ?Sized),
    backup_id: &str,
    target: &(impl AnchorInput + ?Sized),
    at: u64,
    options: crate::DatabaseOptions,
) -> Result<LogicalRestoreReport> {
    let catalogue_root = &catalogue_root.anchor().map_err(backup_io)?;
    let target = &target.anchor().map_err(backup_io)?;
    let catalogue = verify_backup_catalogue(catalogue_root)?;
    let entry = catalogue
        .backups
        .iter()
        .find(|entry| entry.backup_id == backup_id)
        .ok_or_else(|| Error::Archive(format!("backup is not catalogued: {backup_id}")))?;
    let archive = resolve_archive(catalogue_root, &entry.archive_file)?;
    if entry.application_complete {
        let object_manifest = load_object_manifest_for_entry(catalogue_root, entry)?;
        let catalogue_manifest = load_catalogue_manifest_for_entry(catalogue_root, entry)?;
        let (control_archive, control_inventory) =
            load_control_archive_for_entry(catalogue_root, entry)?;
        restore_logical_archive_to_new_root_with_options(
            &archive,
            target,
            at,
            options,
            |staging| {
                restore_catalogues(&catalogue_manifest, staging, options)?;
                restore_object_payloads(catalogue_root, &object_manifest, staging)?;
                restore_control_journal(&control_archive, &control_inventory, staging, options)
            },
            |_| Ok(()),
        )
    } else {
        restore_logical_archive_to_new_root_with_options(
            &archive,
            target,
            at,
            options,
            |_| Ok(()),
            |_| Ok(()),
        )
    }
}

/// Verifies, reading only, that an already published restore still contains
/// the complete immutable payload closure, catalogue state, and replayed
/// control journal of a catalogued backup.
pub fn verify_restored_backup_objects(
    catalogue_root: &(impl AnchorInput + ?Sized),
    backup_id: &str,
    restored_root: &(impl AnchorInput + ?Sized),
) -> Result<()> {
    verify_restored_backup_objects_with_options(
        catalogue_root,
        backup_id,
        restored_root,
        Default::default(),
    )
}

/// Verifies a published application restore under the caller's native policy.
pub fn verify_restored_backup_objects_with_options(
    catalogue_root: &(impl AnchorInput + ?Sized),
    backup_id: &str,
    restored_root: &(impl AnchorInput + ?Sized),
    options: crate::DatabaseOptions,
) -> Result<()> {
    let catalogue_root = &catalogue_root.anchor().map_err(backup_io)?;
    let restored_root = &restored_root.anchor().map_err(backup_io)?;
    let catalogue = verify_backup_catalogue(catalogue_root)?;
    let entry = catalogue
        .backups
        .iter()
        .find(|entry| entry.backup_id == backup_id)
        .ok_or_else(|| Error::Archive(format!("backup is not catalogued: {backup_id}")))?;
    if !entry.application_complete {
        return Err(Error::Archive(
            "logical-only backup cannot verify application-complete restore payloads".into(),
        ));
    }
    let manifest = load_object_manifest_for_entry(catalogue_root, entry)?;
    let restored = LocalObjectStore::open_existing_anchored(&restored_root.join("immutable"))?;
    for object in &manifest.objects {
        let verified = restored.verify(&object.sha256)?;
        if verified.length != object.length {
            return Err(Error::ObjectLengthMismatch {
                expected: object.length,
                actual: verified.length,
            });
        }
    }
    let catalogues = load_catalogue_manifest_for_entry(catalogue_root, entry)?;
    verify_restored_catalogues(&catalogues, restored_root, options)?;
    let (control_archive, control_inventory) =
        load_control_archive_for_entry(catalogue_root, entry)?;
    verify_restored_control_journal(&control_archive, &control_inventory, restored_root, options)
}

/// Reads the control-journal replay progress a restore left in a root, if any.
pub fn control_restore_progress(store: &RrflowKvStore) -> Result<Option<ControlRestoreProgress>> {
    let transaction = store.begin_transaction()?;
    read_control_restore_progress(&*transaction)
}

fn read_control_restore_progress(
    transaction: &dyn crate::StorageTransaction,
) -> Result<Option<ControlRestoreProgress>> {
    get(
        transaction,
        keyspaces::META,
        &keyspaces::control_restore_progress_key(),
    )?
    .map(|bytes| {
        let progress: ControlRestoreProgress = serde_json::from_slice(&bytes)?;
        if progress.format_version != CONTROL_RESTORE_PROGRESS_FORMAT {
            return Err(Error::Archive(format!(
                "unsupported control restore progress version {}",
                progress.format_version
            )));
        }
        Ok(progress)
    })
    .transpose()
}

/// Replays the archived control-journal entries into the staging root, in
/// order, each as one control transition committed together with the replay
/// progress marker. An interrupted replay resumes after the last committed
/// entry; a target whose control state diverges from an entry's recorded
/// before-state fails closed before any further mutation.
fn restore_control_journal(
    archive: &AnchoredPath,
    expected: &ControlArchiveInventory,
    staging_root: &AnchoredPath,
    options: crate::DatabaseOptions,
) -> Result<()> {
    let inspected = inspect_control_archive(archive)?;
    if &inspected != expected {
        return Err(Error::Archive(
            "control-journal archive inventory does not match its catalogue entry".into(),
        ));
    }
    let engine = RrflowKvStore::open_existing_anchored(staging_root, options)?;
    replay_control_entries(archive, expected, &engine, None)?;
    drop(engine);
    verify_restored_control_journal(archive, expected, staging_root, options)
}

/// Applies the archived entries after the recorded progress. `stop_after`
/// ends the replay early without the completion marker; it exists so a test
/// can interrupt a replay the way a crash would and resume it.
fn replay_control_entries(
    archive: &AnchoredPath,
    expected: &ControlArchiveInventory,
    engine: &RrflowKvStore,
    stop_after: Option<u64>,
) -> Result<()> {
    let mut progress = match control_restore_progress(engine)? {
        Some(progress) => {
            if progress.archive_sha256 != expected.archive_sha256
                || progress.entry_count != expected.entry_count
                || progress.applied > progress.entry_count
            {
                return Err(Error::Archive(
                    "restore target carries another control-journal replay".into(),
                ));
            }
            progress
        }
        None => ControlRestoreProgress {
            format_version: CONTROL_RESTORE_PROGRESS_FORMAT,
            archive_sha256: expected.archive_sha256.clone(),
            entry_count: expected.entry_count,
            applied: 0,
            complete: false,
            first_restored_sequence: None,
            last_restored_sequence: None,
        },
    };
    if progress.complete {
        return Ok(());
    }
    let resume_after = progress.applied;
    let mut ordinal = 0u64;
    read_control_archive(archive, |entry| {
        ordinal += 1;
        if ordinal <= resume_after || stop_after.is_some_and(|stop| ordinal > stop) {
            return Ok(());
        }
        let mut transaction = engine.begin_transaction()?;
        let record_key = keyspaces::control_record_key(&entry.key);
        let current = get(&*transaction, keyspaces::META, &record_key)?;
        if current.as_deref().map(digest::sha256_hex) != entry.before_sha256 {
            return Err(Error::Archive(format!(
                "control-journal replay diverged at source sequence {}: {} does not hold its recorded before-state",
                entry.sequence, entry.key
            )));
        }
        let transition = ControlTransition {
            key: entry.key.clone(),
            expected: current,
            replacement: entry.replacement.clone(),
            at: entry.at,
            actor: entry.actor.clone(),
            action: entry.action.clone(),
            request_id: entry.request_id.clone(),
            operation_id: entry.operation_id.clone(),
        };
        let committed =
            prepare_control_batch(&mut *transaction, std::slice::from_ref(&transition))?;
        let restored_sequence = committed
            .first()
            .map(|entry| entry.sequence)
            .ok_or_else(|| Error::Archive("control replay committed no entry".into()))?;
        progress.applied = ordinal;
        progress.complete = ordinal == expected.entry_count;
        progress
            .first_restored_sequence
            .get_or_insert(restored_sequence);
        progress.last_restored_sequence = Some(restored_sequence);
        transaction.put(
            checked_key(keyspaces::META, &keyspaces::control_restore_progress_key())?,
            serde_json::to_vec(&progress)?,
        )?;
        transaction.commit(Durability::Authoritative)?;
        Ok(())
    })?;
    if ordinal != expected.entry_count {
        return Err(Error::Archive(
            "control-journal archive entry count changed between inspection and replay".into(),
        ));
    }
    if stop_after.is_some_and(|stop| stop < expected.entry_count) {
        return Ok(());
    }
    if !progress.complete {
        // An empty scope replays nothing; the marker still records that the
        // archive was applied.
        progress.complete = true;
        let mut transaction = engine.begin_transaction()?;
        transaction.put(
            checked_key(keyspaces::META, &keyspaces::control_restore_progress_key())?,
            serde_json::to_vec(&progress)?,
        )?;
        transaction.commit(Durability::Authoritative)?;
    }
    Ok(())
}

/// Verifies that a restored root's control journal carries every archived
/// entry, contiguously and in order, with the same key, before-state, value,
/// time, actor, action, and identities. Nothing but the replay writes control
/// state into a staged root, so contiguity holds until publication.
fn verify_restored_control_journal(
    archive: &AnchoredPath,
    expected: &ControlArchiveInventory,
    restored_root: &AnchoredPath,
    options: crate::DatabaseOptions,
) -> Result<()> {
    let engine = RrflowKvStore::open_existing_anchored(restored_root, options)?;
    let progress = control_restore_progress(&engine)?.ok_or_else(|| {
        Error::Archive("restored root carries no control-journal replay marker".into())
    })?;
    if progress.archive_sha256 != expected.archive_sha256
        || progress.entry_count != expected.entry_count
        || progress.applied != expected.entry_count
        || !progress.complete
    {
        return Err(Error::Archive(
            "restored root's control-journal replay is incomplete or belongs to another archive"
                .into(),
        ));
    }
    if expected.entry_count == 0 {
        if progress.first_restored_sequence.is_some() || progress.last_restored_sequence.is_some() {
            return Err(Error::Archive(
                "control-journal replay marker is inconsistent".into(),
            ));
        }
        return Ok(());
    }
    let first = progress
        .first_restored_sequence
        .ok_or_else(|| Error::Archive("control-journal replay marker is inconsistent".into()))?;
    if first == 0 || progress.last_restored_sequence != first.checked_add(expected.entry_count - 1)
    {
        return Err(Error::Archive(
            "control-journal replay marker is inconsistent".into(),
        ));
    }
    let mut page = Vec::new().into_iter();
    let mut cursor = first - 1;
    let mut ordinal = 0u64;
    read_control_archive(archive, |entry| {
        let restored = match page.next() {
            Some(restored) => restored,
            None => {
                page = engine
                    .control()
                    .journal_since(cursor, PAGE_SIZE)?
                    .into_iter();
                page.next().ok_or_else(|| {
                    Error::Archive("restored control journal ends before the archive".into())
                })?
            }
        };
        cursor = restored.sequence;
        ordinal += 1;
        if restored.sequence != first + ordinal - 1
            || restored.key != entry.key
            || restored.before_sha256 != entry.before_sha256
            || restored.replacement != entry.replacement
            || restored.at != entry.at
            || restored.actor != entry.actor
            || restored.action != entry.action
            || restored.request_id != entry.request_id
            || restored.operation_id != entry.operation_id
        {
            return Err(Error::Archive(format!(
                "restored control journal differs from the archive at source sequence {}",
                entry.sequence
            )));
        }
        Ok(())
    })?;
    Ok(())
}

/// Streams the control-journal entries of the declared families from the
/// start of the journal to `source_control_sequence` into a content-addressed
/// archive.
fn retain_control_journal(
    engine: &impl StorageEngine,
    catalogue_root: &AnchoredPath,
    source_control_sequence: u64,
    families: &[String],
) -> Result<(String, ControlArchiveInventory)> {
    let archives = catalogue_root.join(CONTROL_ARCHIVE_DIRECTORY);
    archives.create_dir_all().map_err(backup_io)?;
    let id = BACKUP_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let staging = archives.join(format!(".control-archive-{}-{id}.tmp", std::process::id()));
    let mut writer = ControlArchiveWriter::create(&staging, source_control_sequence, families)?;
    let result = (|| {
        let mut after = 0u64;
        while after < source_control_sequence {
            let page = engine.control().journal_since(after, PAGE_SIZE)?;
            if page.is_empty() {
                return Err(Error::Archive(
                    "control journal ended before the captured backup watermark".into(),
                ));
            }
            for entry in page {
                if entry.sequence != after + 1 || entry.sequence > source_control_sequence {
                    return Err(Error::Archive(
                        "control journal changed or became discontinuous during control capture"
                            .into(),
                    ));
                }
                after = entry.sequence;
                if control_family_of(families, &entry.key).is_some() {
                    writer.entry(&entry)?;
                }
            }
        }
        if engine.control().sequence()? != source_control_sequence {
            return Err(Error::Archive(
                "control journal changed while capturing control state".into(),
            ));
        }
        writer.finish()
    })();
    let inventory = match result {
        Ok(inventory) => inventory,
        Err(error) => {
            let _ = staging.remove_file();
            return Err(error);
        }
    };
    let file_name = format!("{}.rrd-control-archive", inventory.archive_sha256);
    let retained_path = archives.join(&file_name);
    if retained_path.try_exists().map_err(backup_io)? {
        let actual = inspect_control_archive(&retained_path)?;
        if actual != inventory {
            let _ = staging.remove_file();
            return Err(Error::Archive(
                "content-addressed control-journal archive conflicts with retained inventory"
                    .into(),
            ));
        }
        staging.remove_file().map_err(backup_io)?;
    } else {
        staging.rename_to(&retained_path).map_err(backup_io)?;
    }
    Ok((
        format!("{CONTROL_ARCHIVE_DIRECTORY}/{file_name}"),
        inventory,
    ))
}

fn load_control_archive_for_entry(
    catalogue_root: &AnchoredPath,
    entry: &BackupEntry,
) -> Result<(AnchoredPath, ControlArchiveInventory)> {
    let file = entry.control_archive_file.as_deref().ok_or_else(|| {
        Error::Archive("application-complete backup has no control-journal archive path".into())
    })?;
    let expected = entry.control_archive.as_ref().ok_or_else(|| {
        Error::Archive(
            "application-complete backup has no control-journal archive inventory".into(),
        )
    })?;
    let path = resolve_control_archive(catalogue_root, file)?;
    let actual = inspect_control_archive(&path)?;
    if &actual != expected {
        return Err(Error::Archive(format!(
            "backup {} control-journal archive inventory does not match catalogue",
            entry.backup_id
        )));
    }
    Ok((path, actual))
}

fn retain_content_addressed_file(
    staging: &AnchoredPath,
    retained: &AnchoredPath,
    inventory: &LogicalArchiveInventory,
) -> Result<()> {
    if retained.try_exists().map_err(backup_io)? {
        let actual = inspect_logical_archive(retained)?;
        if &actual != inventory {
            let _ = staging.remove_file();
            return Err(Error::Archive(
                "content-addressed backup archive conflicts with retained inventory".into(),
            ));
        }
        staging.remove_file().map_err(backup_io)
    } else {
        staging.rename_to(retained).map_err(backup_io)
    }
}

fn retain_catalogue_entry(
    catalogue_root: &AnchoredPath,
    entry: BackupEntry,
) -> Result<BackupEntry> {
    let mut catalogue = load_backup_catalogue(catalogue_root)?;
    deny_pruned_identity_reuse(&catalogue, &entry.backup_id)?;
    if let Some(existing) = catalogue
        .backups
        .iter()
        .find(|existing| existing.backup_id == entry.backup_id)
    {
        if existing == &entry {
            return Ok(existing.clone());
        }
        return Err(Error::Archive("backup identity collision".into()));
    }
    catalogue.revision = catalogue
        .revision
        .checked_add(1)
        .ok_or_else(|| Error::Archive("backup catalogue revision overflow".into()))?;
    catalogue.backups.push(entry.clone());
    catalogue.backups.sort_by(|left, right| {
        (left.created_at, &left.backup_id).cmp(&(right.created_at, &right.backup_id))
    });
    write_catalogue(catalogue_root, catalogue)?;
    Ok(entry)
}

fn object_references_at(engine: &impl StorageEngine, head: u64) -> Result<Vec<ObjectReference>> {
    let mut after = 0u64;
    let mut objects = std::collections::BTreeMap::<String, ObjectReference>::new();
    while after < head {
        let page = engine.runtime().changes_since(after, PAGE_SIZE, None)?;
        if page.head_cursor != head || page.requested_after != after {
            return Err(Error::Archive(
                "runtime watermark changed while collecting backup object references".into(),
            ));
        }
        if page.through_cursor <= after || page.through_cursor > head {
            return Err(Error::Archive(
                "runtime object-reference scan did not advance canonically".into(),
            ));
        }
        for change in page.changes {
            let RuntimeMutation::Object { object } = change.mutation else {
                continue;
            };
            object.validate()?;
            if let Some(existing) = objects.get(&object.sha256) {
                if existing.length != object.length {
                    return Err(Error::Archive(format!(
                        "object digest {} has conflicting lengths in the backup cut",
                        object.sha256
                    )));
                }
            } else {
                if objects.len() == MAX_ARCHIVE_OBJECTS {
                    return Err(Error::Archive(
                        "backup object closure exceeds the v1 entry bound".into(),
                    ));
                }
                objects.insert(object.sha256.clone(), object);
            }
        }
        after = page.through_cursor;
    }
    Ok(objects.into_values().collect())
}

fn retain_catalogues(
    engine: &impl StorageEngine,
    catalogue_root: &AnchoredPath,
) -> Result<(String, CatalogueManifestInventory)> {
    let source_control_sequence = engine.control().sequence()?;
    let mut after = 0u64;
    let mut records = std::collections::BTreeMap::<String, CatalogueRecordSnapshot>::new();
    let mut scopes = std::collections::BTreeSet::<ScopeId>::new();
    while after < source_control_sequence {
        let page = engine.control().journal_since(after, PAGE_SIZE)?;
        if page.is_empty() {
            return Err(Error::Archive(
                "control journal ended before the captured backup watermark".into(),
            ));
        }
        for entry in page {
            if entry.sequence != after + 1 || entry.sequence > source_control_sequence {
                return Err(Error::Archive(
                    "control journal changed or became discontinuous during catalogue capture"
                        .into(),
                ));
            }
            after = entry.sequence;
            let Some(scope) = catalogue_scope(&entry.key)? else {
                continue;
            };
            scopes.insert(scope.clone());
            match entry.replacement {
                Some(value) => {
                    if !records.contains_key(&entry.key)
                        && records.len() == MAX_ARCHIVE_CATALOGUE_RECORDS
                    {
                        return Err(Error::Archive(
                            "backup catalogue closure exceeds the v1 entry bound".into(),
                        ));
                    }
                    records.insert(
                        entry.key.clone(),
                        CatalogueRecordSnapshot {
                            scope,
                            key: entry.key,
                            value,
                        },
                    );
                }
                None => {
                    records.remove(&entry.key);
                }
            }
        }
    }
    if engine.control().sequence()? != source_control_sequence {
        return Err(Error::Archive(
            "control journal changed while capturing catalogue state".into(),
        ));
    }
    let scopes = scopes
        .into_iter()
        .map(|scope| {
            Ok(CatalogueScopeSnapshot {
                revision: engine.runtime().read_stamp(&scope)?.catalog_revision,
                scope,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let payload = CatalogueManifest {
        format_version: CATALOGUE_MANIFEST_VERSION,
        source_control_sequence,
        scopes,
        records: records.into_values().collect(),
    };
    validate_catalogue_manifest(&payload)?;
    let inventory = catalogue_manifest_inventory(&payload)?;
    let stored = StoredCatalogueManifest {
        payload,
        payload_sha256: inventory.manifest_sha256.clone(),
    };
    let manifests = catalogue_root.join(CATALOGUE_MANIFEST_DIRECTORY);
    manifests.create_dir_all().map_err(backup_io)?;
    let file_name = format!("{}.rrd-catalogues.json", inventory.manifest_sha256);
    let retained_path = manifests.join(&file_name);
    if retained_path.try_exists().map_err(backup_io)? {
        let actual = load_catalogue_manifest(&retained_path)?;
        if actual != stored.payload {
            return Err(Error::Archive(
                "content-addressed catalogue manifest conflicts with retained payload".into(),
            ));
        }
    } else {
        let id = BACKUP_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let staging = manifests.join(format!(
            ".catalogue-manifest-{}-{id}.tmp",
            std::process::id()
        ));
        let bytes = serde_json::to_vec_pretty(&stored)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging)
            .map_err(backup_io)?;
        file.write_all(&bytes).map_err(backup_io)?;
        file.sync_all().map_err(backup_io)?;
        drop(file);
        staging.rename_to(&retained_path).map_err(backup_io)?;
    }
    Ok((
        format!("{CATALOGUE_MANIFEST_DIRECTORY}/{file_name}"),
        inventory,
    ))
}

fn catalogue_scope(key: &str) -> Result<Option<ScopeId>> {
    for family in CATALOGUE_CONTROL_FAMILIES {
        if key.len() > family.len() && control_family_covers(family, key) {
            return ScopeId::new(&key[family.len() + 1..])
                .map(Some)
                .map_err(Error::from);
        }
    }
    Ok(None)
}

fn load_catalogue_manifest_for_entry(
    catalogue_root: &AnchoredPath,
    entry: &BackupEntry,
) -> Result<CatalogueManifest> {
    let file = entry.catalogue_manifest_file.as_deref().ok_or_else(|| {
        Error::Archive("application-complete backup has no catalogue manifest path".into())
    })?;
    let expected = entry.catalogue_manifest.as_ref().ok_or_else(|| {
        Error::Archive("application-complete backup has no catalogue manifest inventory".into())
    })?;
    let manifest = load_catalogue_manifest(&resolve_catalogue_manifest(catalogue_root, file)?)?;
    let actual = catalogue_manifest_inventory(&manifest)?;
    if &actual != expected {
        return Err(Error::Archive(format!(
            "backup {} catalogue manifest inventory does not match catalogue",
            entry.backup_id
        )));
    }
    Ok(manifest)
}

fn load_catalogue_manifest(path: &AnchoredPath) -> Result<CatalogueManifest> {
    let bytes = read_manifest_bytes(path)?;
    let stored: StoredCatalogueManifest = serde_json::from_slice(&bytes)?;
    validate_catalogue_manifest(&stored.payload)?;
    let actual = digest::sha256_hex(&serde_json::to_vec(&stored.payload)?);
    if actual != stored.payload_sha256 {
        return Err(Error::Archive(
            "catalogue manifest payload digest does not match".into(),
        ));
    }
    Ok(stored.payload)
}

fn catalogue_manifest_inventory(
    manifest: &CatalogueManifest,
) -> Result<CatalogueManifestInventory> {
    validate_catalogue_manifest(manifest)?;
    let payload_bytes = manifest.records.iter().try_fold(0u64, |total, record| {
        total
            .checked_add(record.value.len() as u64)
            .ok_or_else(|| Error::Archive("catalogue manifest bytes overflowed u64".into()))
    })?;
    Ok(CatalogueManifestInventory {
        format_version: manifest.format_version,
        manifest_sha256: digest::sha256_hex(&serde_json::to_vec(manifest)?),
        source_control_sequence: manifest.source_control_sequence,
        scope_count: manifest.scopes.len() as u64,
        record_count: manifest.records.len() as u64,
        payload_bytes,
    })
}

fn validate_catalogue_manifest(manifest: &CatalogueManifest) -> Result<()> {
    if manifest.format_version != CATALOGUE_MANIFEST_VERSION {
        return Err(Error::Archive(format!(
            "unsupported catalogue manifest version {}",
            manifest.format_version
        )));
    }
    if manifest
        .scopes
        .windows(2)
        .any(|pair| pair[0].scope >= pair[1].scope)
        || manifest
            .records
            .windows(2)
            .any(|pair| pair[0].key >= pair[1].key)
    {
        return Err(Error::Archive(
            "catalogue manifest entries are not uniquely ordered".into(),
        ));
    }
    if manifest.records.len() > MAX_ARCHIVE_CATALOGUE_RECORDS {
        return Err(Error::Archive(
            "backup catalogue closure exceeds the v1 entry bound".into(),
        ));
    }
    let scope_revisions = manifest
        .scopes
        .iter()
        .map(|scope| (&scope.scope, scope.revision))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut record_counts = std::collections::BTreeMap::<&ScopeId, u64>::new();
    for record in &manifest.records {
        if record.value.is_empty() || record.value.len() > 1024 * 1024 {
            return Err(Error::Archive(
                "catalogue manifest record violates control-state bounds".into(),
            ));
        }
        let parsed = catalogue_scope(&record.key)?.ok_or_else(|| {
            Error::Archive("catalogue manifest contains a non-catalogue key".into())
        })?;
        if parsed != record.scope || !scope_revisions.contains_key(&record.scope) {
            return Err(Error::Archive(
                "catalogue manifest key, scope, or inventory differs".into(),
            ));
        }
        *record_counts.entry(&record.scope).or_default() += 1;
    }
    for scope in &manifest.scopes {
        let records = record_counts.get(&scope.scope).copied().unwrap_or_default();
        if scope.revision < records || (scope.revision > 0 && records == 0) {
            return Err(Error::Archive(
                "catalogue scope revision cannot be reconstructed from its records".into(),
            ));
        }
    }
    Ok(())
}

fn restore_catalogues(
    manifest: &CatalogueManifest,
    staging_root: &AnchoredPath,
    options: crate::DatabaseOptions,
) -> Result<()> {
    validate_catalogue_manifest(manifest)?;
    let engine = RrflowKvStore::open_existing_anchored(staging_root, options)?;
    for scope in &manifest.scopes {
        let records = manifest
            .records
            .iter()
            .filter(|record| record.scope == scope.scope)
            .collect::<Vec<_>>();
        let first = records.first().ok_or_else(|| {
            Error::Archive("catalogue revision has no materialized restore record".into())
        })?;
        let mut revision = engine.runtime().read_stamp(&scope.scope)?.catalog_revision;
        if revision > scope.revision {
            return Err(Error::Archive(format!(
                "restored catalogue revision {revision} exceeds {} for {}",
                scope.revision, scope.scope
            )));
        }
        for (ordinal, record) in records.iter().enumerate() {
            let existing = engine.control().get(&record.key)?;
            if (ordinal as u64) < revision {
                if existing.as_deref() != Some(record.value.as_slice()) {
                    return Err(Error::Archive(format!(
                        "partially restored catalogue record {} differs",
                        record.key
                    )));
                }
                continue;
            }
            if existing.is_some() {
                return Err(Error::Archive(format!(
                    "catalogue record {} exists ahead of its revision",
                    record.key
                )));
            }
            engine.control().commit_catalog(
                &scope.scope,
                &ControlTransition {
                    key: record.key.clone(),
                    expected: None,
                    replacement: Some(record.value.clone()),
                    at: 1,
                    actor: "rrd-restore".into(),
                    action: "catalogue.restored".into(),
                    request_id: "rrd-restore".into(),
                    operation_id: "rrd-restore".into(),
                },
            )?;
            revision += 1;
        }
        while revision < scope.revision {
            engine.control().commit_catalog(
                &scope.scope,
                &ControlTransition {
                    key: first.key.clone(),
                    expected: Some(first.value.clone()),
                    replacement: Some(first.value.clone()),
                    at: 1,
                    actor: "rrd-restore".into(),
                    action: "catalogue.revision_restored".into(),
                    request_id: "rrd-restore".into(),
                    operation_id: "rrd-restore".into(),
                },
            )?;
            revision += 1;
        }
    }
    drop(engine);
    verify_restored_catalogues(manifest, staging_root, options)
}

fn verify_restored_catalogues(
    manifest: &CatalogueManifest,
    restored_root: &AnchoredPath,
    options: crate::DatabaseOptions,
) -> Result<()> {
    let engine = RrflowKvStore::open_existing_anchored(restored_root, options)?;
    for record in &manifest.records {
        if engine.control().get(&record.key)?.as_deref() != Some(record.value.as_slice()) {
            return Err(Error::Archive(format!(
                "restored catalogue record {} differs from its backup",
                record.key
            )));
        }
    }
    for scope in &manifest.scopes {
        let actual = engine.runtime().read_stamp(&scope.scope)?.catalog_revision;
        if actual != scope.revision {
            return Err(Error::Archive(format!(
                "restored catalogue revision {actual} differs from {} for {}",
                scope.revision, scope.scope
            )));
        }
    }
    Ok(())
}

fn retain_object_payloads(
    source: &impl ImmutableObjectStore,
    catalogue_root: &AnchoredPath,
    references: &[ObjectReference],
) -> Result<(String, ObjectPayloadManifestInventory)> {
    if references.len() > MAX_ARCHIVE_OBJECTS {
        return Err(Error::Archive(
            "backup object closure exceeds the v1 entry bound".into(),
        ));
    }
    let retained = LocalObjectStore::open_anchored(&catalogue_root.join(OBJECT_PAYLOAD_DIRECTORY))?;
    let mut entries = Vec::with_capacity(references.len());
    let mut payload_bytes = 0u64;
    for reference in references {
        let mut reader = source.open_verified(reference)?;
        let verified =
            retained.put_verified_stream(&reference.sha256, reference.length, reader.as_mut())?;
        if verified.length != reference.length {
            return Err(Error::ObjectLengthMismatch {
                expected: reference.length,
                actual: verified.length,
            });
        }
        payload_bytes = payload_bytes
            .checked_add(reference.length)
            .ok_or_else(|| Error::Archive("backup object payload bytes overflowed u64".into()))?;
        entries.push(ObjectPayloadEntry {
            sha256: reference.sha256.clone(),
            length: reference.length,
        });
    }
    entries.sort_by(|left, right| left.sha256.cmp(&right.sha256));
    let payload = ObjectPayloadManifest {
        format_version: OBJECT_MANIFEST_VERSION,
        objects: entries,
    };
    let payload_sha256 = digest::sha256_hex(&serde_json::to_vec(&payload)?);
    let stored = StoredObjectPayloadManifest {
        payload,
        payload_sha256: payload_sha256.clone(),
    };
    let manifests = catalogue_root.join(OBJECT_MANIFEST_DIRECTORY);
    manifests.create_dir_all().map_err(backup_io)?;
    let file_name = format!("{payload_sha256}.rrd-objects.json");
    let retained_path = manifests.join(&file_name);
    if retained_path.try_exists().map_err(backup_io)? {
        let actual = load_object_manifest(&retained_path)?;
        if actual != stored.payload {
            return Err(Error::Archive(
                "content-addressed object manifest conflicts with retained payload".into(),
            ));
        }
    } else {
        let id = BACKUP_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let staging = manifests.join(format!(".object-manifest-{}-{id}.tmp", std::process::id()));
        let bytes = serde_json::to_vec_pretty(&stored)?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&staging)
            .map_err(backup_io)?;
        file.write_all(&bytes).map_err(backup_io)?;
        file.sync_all().map_err(backup_io)?;
        drop(file);
        staging.rename_to(&retained_path).map_err(backup_io)?;
    }
    Ok((
        format!("{OBJECT_MANIFEST_DIRECTORY}/{file_name}"),
        ObjectPayloadManifestInventory {
            format_version: OBJECT_MANIFEST_VERSION,
            manifest_sha256: payload_sha256,
            object_count: references.len() as u64,
            payload_bytes,
        },
    ))
}

fn verify_object_payloads(catalogue_root: &AnchoredPath, entry: &BackupEntry) -> Result<()> {
    let manifest = load_object_manifest_for_entry(catalogue_root, entry)?;
    let payloads =
        LocalObjectStore::open_existing_anchored(&catalogue_root.join(OBJECT_PAYLOAD_DIRECTORY))?;
    for object in manifest.objects {
        let verified = payloads.verify(&object.sha256)?;
        if verified.length != object.length {
            return Err(Error::ObjectLengthMismatch {
                expected: object.length,
                actual: verified.length,
            });
        }
    }
    Ok(())
}

fn restore_object_payloads(
    catalogue_root: &AnchoredPath,
    manifest: &ObjectPayloadManifest,
    staging_root: &AnchoredPath,
) -> Result<()> {
    let source =
        LocalObjectStore::open_existing_anchored(&catalogue_root.join(OBJECT_PAYLOAD_DIRECTORY))?;
    let target = LocalObjectStore::open_anchored(&staging_root.join("immutable"))?;
    for object in &manifest.objects {
        let verified = source.verify(&object.sha256)?;
        if verified.length != object.length {
            return Err(Error::ObjectLengthMismatch {
                expected: object.length,
                actual: verified.length,
            });
        }
        let reference = ObjectReference::for_verified(
            format!("backup-{}", object.sha256),
            None,
            "application/octet-stream",
            object.sha256.clone(),
            object.length,
            verified.receipt,
        )?;
        let mut reader = source.open_verified(&reference)?;
        target.put_verified_stream(&object.sha256, object.length, reader.as_mut())?;
    }
    Ok(())
}

fn load_object_manifest_for_entry(
    catalogue_root: &AnchoredPath,
    entry: &BackupEntry,
) -> Result<ObjectPayloadManifest> {
    let file = entry.object_manifest_file.as_deref().ok_or_else(|| {
        Error::Archive("application-complete backup has no object manifest path".into())
    })?;
    let expected = entry.object_manifest.as_ref().ok_or_else(|| {
        Error::Archive("application-complete backup has no object manifest inventory".into())
    })?;
    let manifest = load_object_manifest(&resolve_object_manifest(catalogue_root, file)?)?;
    let actual = object_manifest_inventory(&manifest)?;
    if &actual != expected {
        return Err(Error::Archive(format!(
            "backup {} object manifest inventory does not match catalogue",
            entry.backup_id
        )));
    }
    Ok(manifest)
}

fn load_object_manifest(path: &AnchoredPath) -> Result<ObjectPayloadManifest> {
    let bytes = read_manifest_bytes(path)?;
    let stored: StoredObjectPayloadManifest = serde_json::from_slice(&bytes)?;
    validate_object_manifest(&stored.payload)?;
    let actual = digest::sha256_hex(&serde_json::to_vec(&stored.payload)?);
    if actual != stored.payload_sha256 {
        return Err(Error::Archive(
            "object manifest payload digest does not match".into(),
        ));
    }
    Ok(stored.payload)
}

fn read_manifest_bytes(path: &AnchoredPath) -> Result<Vec<u8>> {
    let file = path.open_read().map_err(backup_io)?;
    if file.metadata().map_err(backup_io)?.len() > MAX_ARCHIVE_MANIFEST_BYTES {
        return Err(Error::Archive(
            "backup manifest exceeds the v1 byte bound".into(),
        ));
    }
    let mut bytes = Vec::new();
    file.take(MAX_ARCHIVE_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(backup_io)?;
    if bytes.len() as u64 > MAX_ARCHIVE_MANIFEST_BYTES {
        return Err(Error::Archive(
            "backup manifest exceeds the v1 byte bound".into(),
        ));
    }
    Ok(bytes)
}

fn object_manifest_inventory(
    manifest: &ObjectPayloadManifest,
) -> Result<ObjectPayloadManifestInventory> {
    validate_object_manifest(manifest)?;
    let payload_bytes = manifest.objects.iter().try_fold(0u64, |total, object| {
        total
            .checked_add(object.length)
            .ok_or_else(|| Error::Archive("object manifest payload bytes overflowed u64".into()))
    })?;
    Ok(ObjectPayloadManifestInventory {
        format_version: manifest.format_version,
        manifest_sha256: digest::sha256_hex(&serde_json::to_vec(manifest)?),
        object_count: manifest.objects.len() as u64,
        payload_bytes,
    })
}

fn validate_object_manifest(manifest: &ObjectPayloadManifest) -> Result<()> {
    if manifest.format_version != OBJECT_MANIFEST_VERSION {
        return Err(Error::Archive(format!(
            "unsupported object manifest version {}",
            manifest.format_version
        )));
    }
    if manifest.objects.len() > MAX_ARCHIVE_OBJECTS {
        return Err(Error::Archive(
            "backup object closure exceeds the v1 entry bound".into(),
        ));
    }
    let mut previous: Option<&str> = None;
    for object in &manifest.objects {
        ObjectReference::canonical_key(&object.sha256)?;
        if previous.is_some_and(|prior| prior >= object.sha256.as_str()) {
            return Err(Error::Archive(
                "object manifest entries are not uniquely ordered".into(),
            ));
        }
        previous = Some(&object.sha256);
    }
    Ok(())
}

fn empty_catalogue_payload() -> CataloguePayload {
    CataloguePayload {
        format_version: BACKUP_CATALOGUE_VERSION,
        revision: 0,
        backups: Vec::new(),
        prune_receipts: Vec::new(),
    }
}

/// Retained artifacts exist only for catalogued entries, so a catalogue file
/// that is absent beside them was lost; reading it as empty would hide that.
fn require_no_orphan_artifacts(catalogue_root: &AnchoredPath) -> Result<()> {
    for directory in [
        ARCHIVE_DIRECTORY,
        OBJECT_MANIFEST_DIRECTORY,
        CATALOGUE_MANIFEST_DIRECTORY,
        CONTROL_ARCHIVE_DIRECTORY,
        OBJECT_PAYLOAD_DIRECTORY,
    ] {
        if catalogue_root
            .join(directory)
            .try_exists()
            .map_err(backup_io)?
        {
            return Err(Error::Archive(format!(
                "backup catalogue is absent while retained {directory} exist beside it"
            )));
        }
    }
    Ok(())
}

/// Publishes the empty catalogue before the first artifact is retained, so
/// every retained artifact is younger than the catalogue that owns it and an
/// absent catalogue beside artifacts is always a loss.
fn initialize_catalogue(catalogue_root: &AnchoredPath) -> Result<()> {
    if catalogue_root
        .join(CATALOGUE_FILE)
        .try_exists()
        .map_err(backup_io)?
    {
        return Ok(());
    }
    require_no_orphan_artifacts(catalogue_root)?;
    catalogue_root.create_dir_all().map_err(backup_io)?;
    write_catalogue(
        catalogue_root,
        catalogue_from_payload(empty_catalogue_payload())?,
    )
}

fn catalogue_from_payload(payload: CataloguePayload) -> Result<BackupCatalogue> {
    Ok(BackupCatalogue {
        format_version: payload.format_version,
        revision: payload.revision,
        catalogue_sha256: payload_digest(&payload)?,
        backups: payload.backups,
        prune_receipts: payload.prune_receipts,
    })
}

fn write_catalogue(root: &AnchoredPath, catalogue: BackupCatalogue) -> Result<()> {
    let payload = CataloguePayload {
        format_version: BACKUP_CATALOGUE_VERSION,
        revision: catalogue.revision,
        backups: catalogue.backups,
        prune_receipts: catalogue.prune_receipts,
    };
    validate_entries(&payload.backups)?;
    validate_prune_receipts(&payload.prune_receipts)?;
    let stored = StoredCatalogue {
        payload_sha256: payload_digest(&payload)?,
        payload,
    };
    let bytes = serde_json::to_vec_pretty(&stored)?;
    let id = BACKUP_TEMP_ID.fetch_add(1, Ordering::Relaxed);
    let temporary = root.join(format!(".catalogue-{}-{id}.tmp", std::process::id()));
    let path = root.join(CATALOGUE_FILE);
    crate::filesystem::persist_rename(&temporary, &path, &bytes).map_err(backup_io)
}

fn payload_digest(payload: &CataloguePayload) -> Result<String> {
    Ok(digest::sha256_hex(&serde_json::to_vec(payload)?))
}

fn validate_prune_plan(plan: &BackupPrunePlan) -> Result<()> {
    validate_sha256_text(
        &plan.expected_catalogue_sha256,
        "backup prune expected catalogue",
    )?;
    if plan.prune_candidate_backup_ids.is_empty() {
        return Err(Error::Archive(
            "backup prune requires at least one candidate".into(),
        ));
    }
    validate_ordered_backup_ids(&plan.retained_backup_ids, "retained backup")?;
    validate_ordered_backup_ids(&plan.prune_candidate_backup_ids, "backup prune candidate")
}

fn validate_prune_receipts(receipts: &[BackupPruneReceipt]) -> Result<()> {
    if receipts.len() > MAX_BACKUP_PRUNE_RECEIPTS {
        return Err(Error::Archive(
            "backup prune receipt history exceeds its v1 bound".into(),
        ));
    }
    let mut requests = BTreeSet::new();
    let mut removed_identities = BTreeSet::new();
    for receipt in receipts {
        validate_sha256_text(&receipt.request_sha256, "backup prune request")?;
        validate_sha256_text(
            &receipt.expected_catalogue_sha256,
            "backup prune expected catalogue",
        )?;
        if !requests.insert(receipt.request_sha256.as_str()) {
            return Err(Error::Archive(
                "backup prune receipt request identity is duplicated".into(),
            ));
        }
        validate_ordered_backup_ids(&receipt.retained_backup_ids, "retained backup")?;
        if receipt.removed_backups.is_empty() {
            return Err(Error::Archive(
                "backup prune receipt has no removed backup".into(),
            ));
        }
        validate_entries(&receipt.removed_backups)?;
        for entry in &receipt.removed_backups {
            if receipt
                .retained_backup_ids
                .binary_search(&entry.backup_id)
                .is_ok()
                || !removed_identities.insert(entry.backup_id.as_str())
            {
                return Err(Error::Archive(
                    "backup prune receipt identity is inconsistent".into(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_ordered_backup_ids(values: &[String], name: &str) -> Result<()> {
    let mut previous: Option<&str> = None;
    for value in values {
        validate_sha256_text(value, name)?;
        if previous.is_some_and(|prior| prior >= value.as_str()) {
            return Err(Error::Archive(format!(
                "{name} identities are not uniquely ordered"
            )));
        }
        previous = Some(value);
    }
    Ok(())
}

fn validate_sha256_text(value: &str, name: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(Error::Archive(format!("{name} identity is invalid")));
    }
    Ok(())
}

fn sorted_backup_ids(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

fn deny_pruned_identity_reuse(catalogue: &BackupCatalogue, backup_id: &str) -> Result<()> {
    if catalogue.prune_receipts.iter().any(|receipt| {
        receipt
            .removed_backups
            .iter()
            .any(|entry| entry.backup_id == backup_id)
    }) {
        return Err(Error::Archive(
            "a pruned backup identity cannot be resurrected".into(),
        ));
    }
    Ok(())
}

fn cleanup_pruned_artifacts(
    catalogue_root: &AnchoredPath,
    removed: &[BackupEntry],
    retained: &[BackupEntry],
) -> Result<()> {
    let mut retained_files = BTreeSet::new();
    let mut retained_objects = BTreeSet::new();
    for entry in retained {
        retained_files.insert(entry.archive_file.clone());
        if let Some(path) = &entry.object_manifest_file {
            retained_files.insert(path.clone());
            retained_objects.extend(
                load_object_manifest_for_entry(catalogue_root, entry)?
                    .objects
                    .into_iter()
                    .map(|object| object.sha256),
            );
        }
        if let Some(path) = &entry.catalogue_manifest_file {
            retained_files.insert(path.clone());
        }
        if let Some(path) = &entry.control_archive_file {
            retained_files.insert(path.clone());
        }
    }
    let mut removed_files = BTreeSet::new();
    let mut removed_objects = BTreeSet::new();
    for entry in removed {
        removed_files.insert(entry.archive_file.clone());
        if let Some(path) = &entry.object_manifest_file {
            let manifest_path = resolve_object_manifest(catalogue_root, path)?;
            if manifest_path.try_exists().map_err(backup_io)? {
                removed_objects.extend(
                    load_object_manifest_for_entry(catalogue_root, entry)?
                        .objects
                        .into_iter()
                        .map(|object| object.sha256),
                );
            }
            removed_files.insert(path.clone());
        }
        if let Some(path) = &entry.catalogue_manifest_file {
            removed_files.insert(path.clone());
        }
        if let Some(path) = &entry.control_archive_file {
            removed_files.insert(path.clone());
        }
    }
    let unreachable_objects = removed_objects
        .difference(&retained_objects)
        .cloned()
        .collect::<BTreeSet<_>>();
    LocalObjectStore::open_anchored(&catalogue_root.join(OBJECT_PAYLOAD_DIRECTORY))?
        .reclaim_orphans(&unreachable_objects)?;
    for relative in removed_files.difference(&retained_files) {
        let path = if relative.starts_with(&format!("{ARCHIVE_DIRECTORY}/")) {
            resolve_archive(catalogue_root, relative)?
        } else if relative.starts_with(&format!("{OBJECT_MANIFEST_DIRECTORY}/")) {
            resolve_object_manifest(catalogue_root, relative)?
        } else if relative.starts_with(&format!("{CONTROL_ARCHIVE_DIRECTORY}/")) {
            resolve_control_archive(catalogue_root, relative)?
        } else {
            resolve_catalogue_manifest(catalogue_root, relative)?
        };
        match path.remove_file() {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(backup_io(error)),
        }
    }
    for directory in [
        ARCHIVE_DIRECTORY,
        OBJECT_MANIFEST_DIRECTORY,
        CATALOGUE_MANIFEST_DIRECTORY,
        CONTROL_ARCHIVE_DIRECTORY,
        OBJECT_PAYLOAD_DIRECTORY,
    ] {
        let path = catalogue_root.join(directory);
        if path.try_exists().map_err(backup_io)? && path.metadata().map_err(backup_io)?.is_dir() {
            path.sync_directory().map_err(backup_io)?;
        }
    }
    Ok(())
}

fn validate_entries(entries: &[BackupEntry]) -> Result<()> {
    let mut previous: Option<(u64, &str)> = None;
    let mut identities = std::collections::BTreeSet::new();
    for entry in entries {
        validate_label(&entry.label)?;
        if entry.backup_id.len() != 64
            || !entry
                .backup_id
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            || !identities.insert(&entry.backup_id)
        {
            return Err(Error::Archive(
                "backup catalogue has an invalid or duplicate identity".into(),
            ));
        }
        let order = (entry.created_at, entry.backup_id.as_str());
        if previous.is_some_and(|prior| prior >= order) {
            return Err(Error::Archive(
                "backup catalogue entries are not canonically ordered".into(),
            ));
        }
        previous = Some(order);
        validate_relative_archive(&entry.archive_file)?;
        let common_coverage = entry.claims == BackupCoverage::Included
            && entry.typed_runtime == BackupCoverage::Included
            && entry.projections == BackupCoverage::RebuildRequired
            && entry.invocation_telemetry == BackupCoverage::Excluded
            && entry.snapshot_leases == BackupCoverage::Excluded;
        let logical_only = entry.object_payloads == BackupCoverage::ReferencedOnly
            && entry.catalogues == BackupCoverage::Excluded
            && entry.installed_control == BackupCoverage::Excluded
            && !entry.application_complete
            && entry.object_manifest_file.is_none()
            && entry.object_manifest.is_none()
            && entry.catalogue_manifest_file.is_none()
            && entry.catalogue_manifest.is_none()
            && entry.control_archive_file.is_none()
            && entry.control_archive.is_none();
        let application_complete = entry.object_payloads == BackupCoverage::Included
            && entry.catalogues == BackupCoverage::Included
            && entry.installed_control == BackupCoverage::Included
            && entry.application_complete
            && entry.object_manifest_file.is_some()
            && entry.object_manifest.is_some()
            && entry.catalogue_manifest_file.is_some()
            && entry.catalogue_manifest.is_some()
            && entry.control_archive_file.is_some()
            && entry.control_archive.is_some();
        if !common_coverage || (!logical_only && !application_complete) {
            return Err(Error::Archive(
                "backup v2 coverage declaration is not canonical".into(),
            ));
        }
        if let (Some(file), Some(inventory)) = (&entry.object_manifest_file, &entry.object_manifest)
        {
            validate_relative_object_manifest(file)?;
            if inventory.format_version != OBJECT_MANIFEST_VERSION
                || inventory.manifest_sha256.len() != 64
                || !inventory
                    .manifest_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            {
                return Err(Error::Archive(
                    "backup object manifest inventory is invalid".into(),
                ));
            }
            let expected = format!(
                "{OBJECT_MANIFEST_DIRECTORY}/{}.rrd-objects.json",
                inventory.manifest_sha256
            );
            if file != &expected {
                return Err(Error::Archive(
                    "backup object manifest path does not match its content identity".into(),
                ));
            }
        } else if entry.object_manifest_file.is_some() || entry.object_manifest.is_some() {
            return Err(Error::Archive(
                "backup object manifest path and inventory must be present together".into(),
            ));
        }
        if let (Some(file), Some(inventory)) =
            (&entry.catalogue_manifest_file, &entry.catalogue_manifest)
        {
            validate_relative_catalogue_manifest(file)?;
            if inventory.format_version != CATALOGUE_MANIFEST_VERSION
                || inventory.manifest_sha256.len() != 64
                || !inventory
                    .manifest_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            {
                return Err(Error::Archive(
                    "backup catalogue manifest inventory is invalid".into(),
                ));
            }
            let expected = format!(
                "{CATALOGUE_MANIFEST_DIRECTORY}/{}.rrd-catalogues.json",
                inventory.manifest_sha256
            );
            if file != &expected {
                return Err(Error::Archive(
                    "backup catalogue manifest path does not match its content identity".into(),
                ));
            }
        } else if entry.catalogue_manifest_file.is_some() || entry.catalogue_manifest.is_some() {
            return Err(Error::Archive(
                "backup catalogue manifest path and inventory must be present together".into(),
            ));
        }
        if let (Some(file), Some(inventory)) = (&entry.control_archive_file, &entry.control_archive)
        {
            validate_relative_control_archive(file)?;
            if inventory.format_version != CONTROL_ARCHIVE_VERSION {
                return Err(Error::Archive(
                    "backup control-journal archive inventory is invalid".into(),
                ));
            }
            validate_sha256_text(&inventory.archive_sha256, "backup control-journal archive")?;
            validate_control_families(&inventory.families)?;
            let empty_inventory_consistent = inventory.entry_count != 0
                || (inventory.first_sequence.is_none() && inventory.last_sequence.is_none());
            let populated_inventory_consistent = inventory.entry_count == 0
                || (inventory.first_sequence.is_some()
                    && inventory.last_sequence.is_some()
                    && inventory.last_sequence <= Some(inventory.source_control_sequence)
                    && inventory.first_sequence <= inventory.last_sequence);
            if !empty_inventory_consistent || !populated_inventory_consistent {
                return Err(Error::Archive(
                    "backup control-journal archive inventory is invalid".into(),
                ));
            }
            let expected = format!(
                "{CONTROL_ARCHIVE_DIRECTORY}/{}.rrd-control-archive",
                inventory.archive_sha256
            );
            if file != &expected {
                return Err(Error::Archive(
                    "backup control-journal archive path does not match its content identity"
                        .into(),
                ));
            }
        } else if entry.control_archive_file.is_some() || entry.control_archive.is_some() {
            return Err(Error::Archive(
                "backup control-journal archive path and inventory must be present together".into(),
            ));
        }
        if entry.claims != BackupCoverage::Included
            || entry.typed_runtime != BackupCoverage::Included
            || entry.projections != BackupCoverage::RebuildRequired
            || entry.invocation_telemetry != BackupCoverage::Excluded
            || entry.snapshot_leases != BackupCoverage::Excluded
        {
            return Err(Error::Archive(
                "backup coverage declaration is not canonical".into(),
            ));
        }
    }
    Ok(())
}

fn validate_label(label: &str) -> Result<()> {
    if label.is_empty()
        || label.len() > 128
        || label.trim() != label
        || !label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(Error::Archive(
            "backup label must be 1-128 ASCII alphanumeric, '.', '-', or '_' characters".into(),
        ));
    }
    Ok(())
}

fn validate_relative_archive(value: &str) -> Result<()> {
    let path = Path::new(value);
    let components: Vec<_> = path.components().collect();
    if components.len() != 2
        || components[0] != Component::Normal(ARCHIVE_DIRECTORY.as_ref())
        || !matches!(components[1], Component::Normal(_))
        || !value.ends_with(".rrd-archive")
    {
        return Err(Error::Archive(
            "backup archive path is not canonical".into(),
        ));
    }
    Ok(())
}

fn validate_relative_object_manifest(value: &str) -> Result<()> {
    let path = Path::new(value);
    let components: Vec<_> = path.components().collect();
    if components.len() != 2
        || components[0] != Component::Normal(OBJECT_MANIFEST_DIRECTORY.as_ref())
        || !matches!(components[1], Component::Normal(_))
        || !value.ends_with(".rrd-objects.json")
    {
        return Err(Error::Archive(
            "backup object manifest path is not canonical".into(),
        ));
    }
    Ok(())
}

fn validate_relative_catalogue_manifest(value: &str) -> Result<()> {
    let path = Path::new(value);
    let components: Vec<_> = path.components().collect();
    if components.len() != 2
        || components[0] != Component::Normal(CATALOGUE_MANIFEST_DIRECTORY.as_ref())
        || !matches!(components[1], Component::Normal(_))
        || !value.ends_with(".rrd-catalogues.json")
    {
        return Err(Error::Archive(
            "backup catalogue manifest path is not canonical".into(),
        ));
    }
    Ok(())
}

fn validate_relative_control_archive(value: &str) -> Result<()> {
    let path = Path::new(value);
    let components: Vec<_> = path.components().collect();
    if components.len() != 2
        || components[0] != Component::Normal(CONTROL_ARCHIVE_DIRECTORY.as_ref())
        || !matches!(components[1], Component::Normal(_))
        || !value.ends_with(".rrd-control-archive")
    {
        return Err(Error::Archive(
            "backup control-journal archive path is not canonical".into(),
        ));
    }
    Ok(())
}

fn resolve_archive(root: &AnchoredPath, value: &str) -> Result<AnchoredPath> {
    validate_relative_archive(value)?;
    Ok(root.join(value))
}

fn resolve_control_archive(root: &AnchoredPath, value: &str) -> Result<AnchoredPath> {
    validate_relative_control_archive(value)?;
    Ok(root.join(value))
}

fn resolve_object_manifest(root: &AnchoredPath, value: &str) -> Result<AnchoredPath> {
    validate_relative_object_manifest(value)?;
    Ok(root.join(value))
}

fn resolve_catalogue_manifest(root: &AnchoredPath, value: &str) -> Result<AnchoredPath> {
    validate_relative_catalogue_manifest(value)?;
    Ok(root.join(value))
}

fn backup_io(error: std::io::Error) -> Error {
    Error::Archive(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn oversized_manifest_is_refused_without_reading_its_payload() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("oversized-manifest.json");
        let file = fs::File::create(&path).unwrap();
        file.set_len(MAX_ARCHIVE_MANIFEST_BYTES + 1).unwrap();
        drop(file);
        let path = AnchoredPath::from_path(&path).unwrap();
        for error in [
            load_object_manifest(&path).unwrap_err(),
            load_catalogue_manifest(&path).unwrap_err(),
        ] {
            assert!(error.to_string().contains("exceeds the v1 byte bound"));
        }
        assert_eq!(
            path.metadata().unwrap().len(),
            MAX_ARCHIVE_MANIFEST_BYTES + 1
        );
    }

    fn transition(
        key: &str,
        expected: Option<&[u8]>,
        replacement: Option<&[u8]>,
        at: u64,
    ) -> ControlTransition {
        ControlTransition {
            key: key.into(),
            expected: expected.map(<[u8]>::to_vec),
            replacement: replacement.map(<[u8]>::to_vec),
            at,
            actor: "replay-test".into(),
            action: "test.transition".into(),
            request_id: format!("request-{at}"),
            operation_id: format!("operation-{at}"),
        }
    }

    fn families() -> Vec<String> {
        vec![
            "server/state/install".into(),
            "server/state/one/audit".into(),
        ]
    }

    /// A source with a key that moves three times and another that is created
    /// and deleted, archived under the declared families.
    fn archived_source(root: &Path) -> (AnchoredPath, ControlArchiveInventory) {
        let source = RrflowKvStore::create_new(&root.join("source")).unwrap();
        let control = source.control();
        control
            .commit(&transition(
                "server/state/install/record",
                None,
                Some(b"v1"),
                1,
            ))
            .unwrap();
        control
            .commit(&transition(
                "server/state/one/audit/a",
                None,
                Some(b"a1"),
                2,
            ))
            .unwrap();
        control
            .commit(&transition(
                "server/state/install/record",
                Some(b"v1"),
                Some(b"v2"),
                3,
            ))
            .unwrap();
        control
            .commit(&transition(
                "server/state/one/session/s",
                None,
                Some(b"lease"),
                4,
            ))
            .unwrap();
        control
            .commit(&transition(
                "server/state/one/audit/a",
                Some(b"a1"),
                None,
                5,
            ))
            .unwrap();
        control
            .commit(&transition(
                "server/state/install/record",
                Some(b"v2"),
                Some(b"v3"),
                6,
            ))
            .unwrap();
        let catalogue_root = root.join("catalogue");
        fs::create_dir_all(&catalogue_root).unwrap();
        let (file, inventory) = retain_control_journal(
            &source,
            &AnchoredPath::from_path(&catalogue_root).unwrap(),
            6,
            &families(),
        )
        .unwrap();
        assert_eq!(inventory.entry_count, 5);
        (
            AnchoredPath::from_path(&catalogue_root.join(file)).unwrap(),
            inventory,
        )
    }

    #[test]
    fn an_interrupted_replay_resumes_after_its_last_committed_entry() {
        let root = tempfile::tempdir().unwrap();
        let (archive, inventory) = archived_source(root.path());
        let target_root = root.path().join("target");
        let target = RrflowKvStore::create_new(&target_root).unwrap();

        replay_control_entries(&archive, &inventory, &target, Some(3)).unwrap();
        let partial = control_restore_progress(&target).unwrap().unwrap();
        assert_eq!(partial.applied, 3);
        assert!(!partial.complete);
        assert_eq!(partial.first_restored_sequence, Some(1));
        assert_eq!(partial.last_restored_sequence, Some(3));
        assert_eq!(
            target.control().get("server/state/install/record").unwrap(),
            Some(b"v2".to_vec())
        );
        drop(target);
        assert!(
            verify_restored_control_journal(
                &archive,
                &inventory,
                &AnchoredPath::from_path(&target_root).unwrap(),
                Default::default(),
            )
            .unwrap_err()
            .to_string()
            .contains("incomplete"),
            "an interrupted replay does not verify"
        );

        restore_control_journal(
            &archive,
            &inventory,
            &AnchoredPath::from_path(&target_root).unwrap(),
            Default::default(),
        )
        .unwrap();
        let target = RrflowKvStore::open_existing(&target_root).unwrap();
        let progress = control_restore_progress(&target).unwrap().unwrap();
        assert!(progress.complete);
        assert_eq!(progress.applied, 5);
        assert_eq!(progress.last_restored_sequence, Some(5));
        assert_eq!(
            target.control().sequence().unwrap(),
            5,
            "no entry is replayed twice"
        );
        assert_eq!(
            target.control().get("server/state/install/record").unwrap(),
            Some(b"v3".to_vec())
        );
        assert_eq!(
            target.control().get("server/state/one/audit/a").unwrap(),
            None
        );
        assert_eq!(
            target.control().get("server/state/one/session/s").unwrap(),
            None
        );
        drop(target);

        // A completed replay is idempotent.
        restore_control_journal(
            &archive,
            &inventory,
            &AnchoredPath::from_path(&target_root).unwrap(),
            Default::default(),
        )
        .unwrap();
        let target = RrflowKvStore::open_existing(&target_root).unwrap();
        assert_eq!(target.control().sequence().unwrap(), 5);
    }

    #[test]
    fn a_target_whose_control_state_diverges_fails_closed() {
        let root = tempfile::tempdir().unwrap();
        let (archive, inventory) = archived_source(root.path());
        let target_root = root.path().join("target");
        let target = RrflowKvStore::create_new(&target_root).unwrap();
        target
            .control()
            .commit(&transition(
                "server/state/install/record",
                None,
                Some(b"foreign"),
                9,
            ))
            .unwrap();
        drop(target);
        let error = restore_control_journal(
            &archive,
            &inventory,
            &AnchoredPath::from_path(&target_root).unwrap(),
            Default::default(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("diverged at source sequence 1"));
        let target = RrflowKvStore::open_existing(&target_root).unwrap();
        assert_eq!(
            target.control().sequence().unwrap(),
            1,
            "nothing was applied"
        );
        assert!(control_restore_progress(&target).unwrap().is_none());
    }

    #[test]
    fn a_marker_from_another_archive_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let (archive, inventory) = archived_source(root.path());
        let target_root = root.path().join("target");
        let target = RrflowKvStore::create_new(&target_root).unwrap();
        replay_control_entries(&archive, &inventory, &target, Some(2)).unwrap();
        drop(target);
        let mut other = inventory.clone();
        other.archive_sha256 = "0".repeat(64);
        let target = RrflowKvStore::open_existing(&target_root).unwrap();
        assert!(replay_control_entries(&archive, &other, &target, None)
            .unwrap_err()
            .to_string()
            .contains("another control-journal replay"));
    }
}
