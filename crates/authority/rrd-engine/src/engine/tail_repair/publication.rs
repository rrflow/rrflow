//! Accounted copy and resumable publication. The damaged input is never edited.
use super::*;
use crate::engine::estate_layout::{instance_operation_root, RESTORES_DIRECTORY_NAME};
use crate::engine::installation::ProjectLocator;
use crate::engine::restore_publication::PublicationLease;
use crate::engine::token_key::{
    create_private_directory_anchored, read_private_anchored, write_private_anchored,
};
use rrd_store::filesystem::{AnchoredPath, OpenOptions};
use rrd_store::RrflowKvInspector;
use std::io::{Read, Write};

const AUDIT_FILE: &str = "RRD.REPAIR.json";
const MAX_PLAN_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairCopyFile {
    pub relative_path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairCopyInventory {
    pub directories: Vec<String>,
    pub files: Vec<RepairCopyFile>,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairPublication {
    pub active_root: String,
    pub candidate_root: String,
    pub quarantine_root: String,
    pub mechanism: String,
    pub fallback: String,
    pub locator_action: String,
    pub same_filesystem_required: bool,
    pub worst_case_logical_write_bytes: u64,
    pub before: RepairCopyInventory,
    pub before_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Stage {
    Copying,
    CandidateReady,
    Quarantined,
    Published,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    version: u16,
    plan_sha256: String,
    stage: Stage,
    receipt: Option<InstalledTailRepairReceipt>,
    sha256: String,
}
impl Intent {
    fn digest(&self) -> Result<String> {
        json_sha256(&(self.version, &self.plan_sha256, self.stage, &self.receipt))
    }
}

struct Work {
    limits: rrd_contract::StorageLimits,
    files: u64,
    bytes: u64,
    started: Instant,
}
impl Work {
    fn new(locator: &ProjectLocator) -> Self {
        Self {
            limits: locator.storage.clone(),
            files: 0,
            bytes: 0,
            started: Instant::now(),
        }
    }
    fn tick(&self) -> Result<()> {
        if self.started.elapsed().as_millis() > self.limits.recovery_elapsed_ms as u128 {
            return Err(ServiceError::Storage(
                "repair exceeds configured elapsed work".into(),
            ));
        }
        Ok(())
    }
    fn charge(&mut self, bytes: u64) -> Result<()> {
        self.tick()?;
        self.files = self
            .files
            .checked_add(1)
            .ok_or(ServiceError::ProjectBindingMismatch)?;
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or(ServiceError::ProjectBindingMismatch)?;
        if self.files > self.limits.recovery_files || self.bytes > self.limits.recovery_read_bytes {
            return Err(ServiceError::Storage(
                "repair exceeds configured file/read budget".into(),
            ));
        }
        Ok(())
    }
}

pub(super) fn preview(project: &Path, bound: &BoundRepair) -> Result<RepairPublication> {
    let (project, locator, _) = read_locator(project)?;
    let root = safe_join(&project, &locator.storage_root)?;
    let before = inventory(&root, &mut Work::new(&locator))?;
    let before_sha256 = json_sha256(&before)?;
    let (candidate_root, quarantine_root) = destinations(&locator, &before_sha256)?;
    for path in [&candidate_root, &quarantine_root] {
        if exists(&safe_join(&project, path)?)? {
            return Err(ServiceError::Contract(
                "repair destination already exists; resume its approved plan".into(),
            ));
        }
    }
    if bound.target != locator.identity {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    let publication = RepairPublication {
        active_root: locator.storage_root,
        candidate_root,
        quarantine_root,
        mechanism: "accounted_byte_copy".into(),
        fallback: "none".into(),
        locator_action: "retain_binding_and_publish_under_existing_root_name".into(),
        same_filesystem_required: true,
        worst_case_logical_write_bytes: before
            .total_bytes
            .checked_add(2 * MAX_PLAN_BYTES as u64)
            .ok_or(ServiceError::ProjectBindingMismatch)?,
        before,
        before_sha256,
    };
    if serde_json::to_vec(&publication).map_err(json_error)?.len() > MAX_PLAN_BYTES / 2 {
        return Err(ServiceError::Contract(
            "repair inventory exceeds bounded plan size".into(),
        ));
    }
    Ok(publication)
}

fn destinations(locator: &ProjectLocator, digest: &str) -> Result<(String, String)> {
    let parent = Path::new(&locator.storage_root)
        .parent()
        .and_then(Path::to_str)
        .ok_or(ServiceError::ProjectBindingMismatch)?
        .replace('\\', "/");
    Ok((
        format!("{parent}/repair-{}", &digest[..24]),
        format!(
            ".rrflow/rrd/restores/{}/repair-before-{digest}",
            locator.identity.instance_id
        ),
    ))
}

pub(super) fn apply(
    project_path: &Path,
    executable: &Path,
    plan: &InstalledTailRepairPlan,
) -> Result<InstalledTailRepairReceipt> {
    apply_inner(project_path, executable, plan, &mut |_| Ok(()))
}

#[cfg(test)]
pub(in crate::engine) fn apply_with_observer(
    project_path: &Path,
    executable: &Path,
    plan: &InstalledTailRepairPlan,
    observer: &mut dyn FnMut(&'static str) -> Result<()>,
) -> Result<InstalledTailRepairReceipt> {
    apply_inner(project_path, executable, plan, observer)
}

fn apply_inner(
    project_path: &Path,
    executable: &Path,
    plan: &InstalledTailRepairPlan,
    observer: &mut dyn FnMut(&'static str) -> Result<()>,
) -> Result<InstalledTailRepairReceipt> {
    let (project, locator, bytes) = read_locator(project_path)?;
    let executable_sha256 =
        crate::engine::restore_publication::require_distribution_binding(&locator, executable)?;
    let publication = &plan.publication;
    let before_sha256 = json_sha256(&publication.before)?;
    let destinations = destinations(&locator, &before_sha256)?;
    if plan.target != locator.identity
        || digest::sha256_hex(&bytes) != plan.locator_sha256
        || executable_sha256 != plan.executable_sha256
        || locator.configuration_sha256 != plan.configuration_sha256
        || publication.active_root != locator.storage_root
        || publication.before_sha256 != before_sha256
        || (&publication.candidate_root, &publication.quarantine_root)
            != (&destinations.0, &destinations.1)
        || publication.mechanism != "accounted_byte_copy"
        || publication.fallback != "none"
        || publication.locator_action != "retain_binding_and_publish_under_existing_root_name"
        || !publication.same_filesystem_required
        || publication.worst_case_logical_write_bytes
            != publication
                .before
                .total_bytes
                .checked_add(2 * MAX_PLAN_BYTES as u64)
                .ok_or(ServiceError::ProjectBindingMismatch)?
    {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    let active = safe_join(&project, &publication.active_root)?;
    let candidate = safe_join(&project, &publication.candidate_root)?;
    let quarantine = safe_join(&project, &publication.quarantine_root)?;
    let restores = instance_operation_root(
        &project,
        RESTORES_DIRECTORY_NAME,
        &locator.identity.instance_id,
    )?;
    private_directories(&restores)?;
    let _lease = PublicationLease::acquire_at(&restores)?;
    let intent_path = restores.join(format!("repair-{}.json", plan.plan_sha256));
    let receipt_key = format!(
        "server/state/{}/repair/{}",
        locator.identity.instance_id, plan.plan_sha256
    );
    if !exists(&intent_path)? && exists(&active)? {
        // Completed retries read the canonical control journal before comparing
        // a pre-repair inventory that subsequent legitimate writes have changed.
        if let Ok(bound) = BoundRepair::open(project_path, executable) {
            require_bound(&bound, plan)?;
            if let Some(bytes) = bound.repair.control_record(&receipt_key)? {
                let mut receipt: InstalledTailRepairReceipt =
                    serde_json::from_slice(&bytes).map_err(json_error)?;
                if receipt.plan_sha256 != plan.plan_sha256 || receipt.target != plan.target {
                    return Err(ServiceError::ProjectBindingMismatch);
                }
                receipt.idempotent_replay = true;
                return Ok(receipt);
            }
        }
    }
    let mut intent = if exists(&intent_path)? {
        read_intent(&intent_path, plan)?
    } else {
        let bound = BoundRepair::open(project_path, executable)?;
        require_bound(&bound, plan)?;
        if bound.repair.plan()? != plan.storage
            || inventory(&active, &mut Work::new(&locator))? != publication.before
            || exists(&candidate)?
            || exists(&quarantine)?
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        let mut intent = Intent {
            version: 1,
            plan_sha256: plan.plan_sha256.clone(),
            stage: Stage::Copying,
            receipt: None,
            sha256: String::new(),
        };
        write_intent(&intent_path, &mut intent, false)?;
        intent
    };
    if intent.stage == Stage::Copying {
        let bound = BoundRepair::open(project_path, executable)?;
        require_bound(&bound, plan)?;
        let mut work = Work::new(&locator);
        if bound.repair.plan()? != plan.storage
            || inventory(&active, &mut work)? != publication.before
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        copy_candidate(&active, &candidate, &publication.before, &mut work)?;
        observer("candidate-copied")?;
        let repair = RrflowKvTailRepair::open_anchored(
            &candidate,
            crate::engine::core::installed_storage_options(&locator.storage)?,
        )?;
        let runtime_cursor = repair.inspection().runtime_cursor;
        let claim_sequence = repair.inspection().claim_sequence;
        let storage = repair.apply(&plan.storage, &plan.storage.plan_sha256)?;
        let inspector = RrflowKvInspector::open_anchored_with_options(
            &candidate,
            crate::engine::core::installed_storage_options(&locator.storage)?,
        )?;
        if inspector.inspection().runtime_cursor != runtime_cursor
            || inspector.inspection().claim_sequence != claim_sequence
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        drop(inspector);
        let after = inventory(&candidate, &mut work)?;
        let receipt = InstalledTailRepairReceipt {
            plan_sha256: plan.plan_sha256.clone(),
            target: plan.target.clone(),
            runtime_cursor,
            claim_sequence,
            storage,
            quarantine_root: publication.quarantine_root.clone(),
            before_sha256: publication.before_sha256.clone(),
            candidate_sha256: json_sha256(&after)?,
            idempotent_replay: false,
        };
        write_private_anchored(
            &candidate.join(AUDIT_FILE),
            &serde_json::to_vec(&receipt).map_err(json_error)?,
        )
        .map_err(io_error)?;
        intent.receipt = Some(receipt);
        intent.stage = Stage::CandidateReady;
        write_intent(&intent_path, &mut intent, true)?;
        drop(bound);
    }
    if intent.stage == Stage::CandidateReady {
        verify_candidate(&candidate, &locator, &intent)?;
        match (exists(&active)?, exists(&quarantine)?) {
            (true, false) => {
                let bound = BoundRepair::open(project_path, executable)?;
                require_bound(&bound, plan)?;
                if inventory(&active, &mut Work::new(&locator))? != publication.before {
                    return Err(ServiceError::ProjectBindingMismatch);
                }
                drop(bound);
                active.rename_new_to(&quarantine).map_err(io_error)?;
                sync_parent(&active)?;
                sync_parent(&quarantine)?;
                observer("active-quarantined")?;
            }
            (false, true) => {
                if inventory(&quarantine, &mut Work::new(&locator))? != publication.before {
                    return Err(ServiceError::ProjectBindingMismatch);
                }
            }
            _ => return Err(ServiceError::ProjectBindingMismatch),
        }
        intent.stage = Stage::Quarantined;
        write_intent(&intent_path, &mut intent, true)?;
    }
    if intent.stage == Stage::Quarantined {
        if inventory(&quarantine, &mut Work::new(&locator))? != publication.before {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        match (exists(&candidate)?, exists(&active)?) {
            (true, false) => {
                verify_candidate(&candidate, &locator, &intent)?;
                candidate.rename_new_to(&active).map_err(io_error)?;
                sync_parent(&candidate)?;
                sync_parent(&active)?;
                observer("candidate-published")?;
            }
            (false, true) => {
                verify_candidate(&active, &locator, &intent)?;
            }
            _ => return Err(ServiceError::ProjectBindingMismatch),
        }
        intent.stage = Stage::Published;
        write_intent(&intent_path, &mut intent, true)?;
    }
    let receipt = intent
        .receipt
        .clone()
        .ok_or(ServiceError::ProjectBindingMismatch)?;
    // A bounded intent and candidate audit receipt survive before publication.
    // The engine-owned control journal records final completion idempotently.
    let engine = RrdEngine::open_installed(project_path, executable)?;
    let receipt_bytes = serde_json::to_vec(&receipt).map_err(json_error)?;
    match engine.storage.control().get(&receipt_key)? {
        Some(existing) if existing == receipt_bytes => {}
        Some(_) => return Err(ServiceError::IdempotencyConflict),
        None => {
            let at = crate::engine::clock::observe_required(
                &crate::engine::clock::HostSystemClock,
                "repair.publication",
            )?
            .observed_at_unix_ms;
            engine.storage.control().commit(&ControlTransition {
                key: receipt_key,
                expected: None,
                replacement: Some(receipt_bytes),
                at,
                actor: "rrflow:tail-repair".into(),
                action: "repair.published".into(),
                request_id: format!("repair-{}", &plan.plan_sha256[..24]),
                operation_id: format!("repair-{}", &plan.plan_sha256[..24]),
            })?;
        }
    }
    drop(engine);
    observer("receipt-committed")?;
    let report = RrdEngine::inspect_installed(project_path, executable)?;
    if report.status != crate::InstallationVerificationStatus::Passed {
        return Err(ServiceError::Storage(
            "published repair failed installed verification".into(),
        ));
    }
    if read_intent(&intent_path, plan)? != intent {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    intent_path.remove_file().map_err(io_error)?;
    sync_parent(&intent_path)?;
    Ok(receipt)
}

fn require_bound(bound: &BoundRepair, plan: &InstalledTailRepairPlan) -> Result<()> {
    if bound.target != plan.target
        || bound.locator_sha256 != plan.locator_sha256
        || bound.executable_sha256 != plan.executable_sha256
        || bound.configuration_sha256 != plan.configuration_sha256
        || bound.installed_record_sha256 != plan.installed_record_sha256
    {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(())
}

fn verify_candidate(root: &AnchoredPath, locator: &ProjectLocator, intent: &Intent) -> Result<()> {
    let receipt = intent
        .receipt
        .as_ref()
        .ok_or(ServiceError::ProjectBindingMismatch)?;
    let bytes = read_private_anchored(&root.join(AUDIT_FILE), 64 * 1024).map_err(io_error)?;
    if bytes != serde_json::to_vec(receipt).map_err(json_error)? {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    let inspector = RrflowKvInspector::open_anchored_with_options(
        root,
        crate::engine::core::installed_storage_options(&locator.storage)?,
    )?;
    if inspector.inspection().runtime_cursor != receipt.runtime_cursor
        || inspector.inspection().claim_sequence != receipt.claim_sequence
    {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    let mut actual = inventory(root, &mut Work::new(locator))?;
    let audit = actual
        .files
        .iter()
        .position(|f| f.relative_path == AUDIT_FILE)
        .ok_or(ServiceError::ProjectBindingMismatch)?;
    actual.total_bytes -= actual.files.remove(audit).bytes;
    if json_sha256(&actual)? != receipt.candidate_sha256 {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(())
}

fn inventory(root: &AnchoredPath, work: &mut Work) -> Result<RepairCopyInventory> {
    let mut pending = vec![root.clone()];
    let mut result = RepairCopyInventory {
        directories: Vec::new(),
        files: Vec::new(),
        total_bytes: 0,
    };
    while let Some(directory) = pending.pop() {
        for entry in directory.entries().map_err(io_error)? {
            let entry = entry.map_err(io_error)?;
            work.charge(0)?;
            let relative = entry
                .relative_path()
                .strip_prefix(root.relative_path())
                .map_err(|_| ServiceError::ProjectBindingMismatch)?
                .to_str()
                .ok_or(ServiceError::ProjectBindingMismatch)?
                .replace('\\', "/");
            let metadata = entry.metadata().map_err(io_error)?;
            if metadata.is_dir() && !metadata.file_type().is_symlink() {
                result.directories.push(relative);
                pending.push(entry);
            } else if metadata.is_file() && !metadata.file_type().is_symlink() {
                let file = hash_file(&entry, &relative, work)?;
                result.total_bytes = result
                    .total_bytes
                    .checked_add(file.bytes)
                    .ok_or(ServiceError::ProjectBindingMismatch)?;
                result.files.push(file);
            } else {
                return Err(ServiceError::ProjectBindingMismatch);
            }
        }
    }
    result.directories.sort();
    result
        .files
        .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(result)
}

fn hash_file(path: &AnchoredPath, relative: &str, work: &mut Work) -> Result<RepairCopyFile> {
    let mut file = path.open_read().map_err(io_error)?;
    let bytes = file.metadata().map_err(io_error)?.len();
    work.charge(bytes)?;
    let mut remaining = bytes;
    let mut buffer = [0u8; 64 * 1024];
    let mut digest = rrd_core::digest::Sha256::new();
    while remaining != 0 {
        work.tick()?;
        let n = remaining.min(buffer.len() as u64) as usize;
        file.read_exact(&mut buffer[..n]).map_err(io_error)?;
        digest.update(&buffer[..n]);
        remaining -= n as u64;
    }
    if file.metadata().map_err(io_error)?.len() != bytes {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(RepairCopyFile {
        relative_path: relative.into(),
        bytes,
        sha256: digest.finalize_hex(),
    })
}

fn copy_candidate(
    source: &AnchoredPath,
    target: &AnchoredPath,
    before: &RepairCopyInventory,
    work: &mut Work,
) -> Result<()> {
    private_directories(target)?;
    let existing = inventory(target, work)?;
    if existing
        .directories
        .iter()
        .any(|p| !before.directories.contains(p))
        || existing.files.iter().any(|f| {
            f.relative_path != AUDIT_FILE
                && !before
                    .files
                    .iter()
                    .any(|p| p.relative_path == f.relative_path)
        })
    {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    let audit = target.join(AUDIT_FILE);
    if exists(&audit)? {
        read_private_anchored(&audit, 64 * 1024).map_err(io_error)?;
        audit.remove_file().map_err(io_error)?;
    }
    for directory in &before.directories {
        private_directories(&target.join(directory))?;
    }
    for expected in &before.files {
        if expected.relative_path == AUDIT_FILE {
            continue;
        }
        let destination = target.join(&expected.relative_path);
        if exists(&destination)? {
            if hash_file(&destination, &expected.relative_path, work)? == *expected {
                continue;
            }
            destination.remove_file().map_err(io_error)?;
        }
        work.charge(expected.bytes)?;
        let mut input = source
            .join(&expected.relative_path)
            .open_read()
            .map_err(io_error)?;
        write_private_anchored(&destination, &[]).map_err(io_error)?;
        let mut output = OpenOptions::new()
            .write(true)
            .open(&destination)
            .map_err(io_error)?;
        let mut remaining = expected.bytes;
        let mut buffer = [0u8; 64 * 1024];
        let mut digest = rrd_core::digest::Sha256::new();
        while remaining != 0 {
            work.tick()?;
            let n = remaining.min(buffer.len() as u64) as usize;
            input.read_exact(&mut buffer[..n]).map_err(io_error)?;
            output.write_all(&buffer[..n]).map_err(io_error)?;
            digest.update(&buffer[..n]);
            remaining -= n as u64;
        }
        if input.metadata().map_err(io_error)?.len() != expected.bytes
            || digest.finalize_hex() != expected.sha256
        {
            return Err(ServiceError::ProjectBindingMismatch);
        }
        output.sync_all().map_err(io_error)?;
    }
    for directory in before.directories.iter().rev() {
        target.join(directory).sync_directory().map_err(io_error)?;
    }
    target.sync_directory().map_err(io_error)
}

fn private_directories(path: &AnchoredPath) -> Result<()> {
    if exists(path)? {
        path.pin_directory().map_err(io_error)?;
        return Ok(());
    }
    let parent = path.parent().ok_or(ServiceError::ProjectBindingMismatch)?;
    private_directories(&parent)?;
    create_private_directory_anchored(path).map_err(io_error)?;
    Ok(())
}
fn exists(path: &AnchoredPath) -> Result<bool> {
    match path.metadata() {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(io_error(e)),
    }
}
fn sync_parent(path: &AnchoredPath) -> Result<()> {
    path.parent()
        .ok_or(ServiceError::ProjectBindingMismatch)?
        .sync_directory()
        .map_err(io_error)
}
fn write_intent(path: &AnchoredPath, intent: &mut Intent, replace: bool) -> Result<()> {
    intent.sha256 = intent.digest()?;
    let bytes = serde_json::to_vec(intent).map_err(json_error)?;
    if bytes.len() > 64 * 1024 {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    let temporary = path.with_extension("json.pending");
    if exists(&temporary)? {
        read_private_anchored(&temporary, 64 * 1024).map_err(io_error)?;
        temporary.remove_file().map_err(io_error)?;
    }
    write_private_anchored(&temporary, &bytes).map_err(io_error)?;
    if replace {
        temporary.rename_to(path).map_err(io_error)?;
    } else {
        temporary.rename_new_to(path).map_err(io_error)?;
    }
    sync_parent(path)
}
fn read_intent(path: &AnchoredPath, plan: &InstalledTailRepairPlan) -> Result<Intent> {
    let bytes = read_private_anchored(path, 64 * 1024).map_err(io_error)?;
    let intent: Intent = serde_json::from_slice(&bytes).map_err(json_error)?;
    if intent.version != 1
        || intent.plan_sha256 != plan.plan_sha256
        || intent.sha256 != intent.digest()?
    {
        return Err(ServiceError::ProjectBindingMismatch);
    }
    Ok(intent)
}
fn io_error(error: std::io::Error) -> ServiceError {
    ServiceError::Storage(error.to_string())
}
fn json_error(error: serde_json::Error) -> ServiceError {
    ServiceError::Contract(error.to_string())
}
