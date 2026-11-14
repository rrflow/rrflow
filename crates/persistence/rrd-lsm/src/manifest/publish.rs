//! Publishing manifests, named checkpoints and the current pointer.

use super::*;

impl ManifestStore {
    pub fn publish(&self, manifest: &Manifest, expected: Option<&str>) -> Result<CurrentPointer> {
        self.ensure_publication_ready()?;
        manifest.validate()?;
        let current = self.current()?;
        let actual = current
            .as_ref()
            .map(|(pointer, _)| pointer.manifest.clone());
        if actual.as_deref() != expected {
            return Err(Error::ManifestConflict {
                expected: expected.map(str::to_owned),
                actual,
            });
        }
        match current {
            None if manifest.generation == 1 && manifest.parent.is_none() => {}
            Some((pointer, _))
                if manifest.generation == pointer.generation + 1
                    && manifest.parent.as_deref() == Some(pointer.manifest.as_str()) => {}
            _ => {
                return Err(Error::InvalidManifest(
                    "manifest generation/parent does not extend CURRENT".into(),
                ))
            }
        }
        self.publication_result(self.persist_manifest(manifest))?;
        let pointer = CurrentPointer::new(manifest);
        self.publication_result(self.publish_pointer(&pointer))?;
        Ok(pointer)
    }

    /// Restart-only ceiling shared by checkpoint creation, listing and release.
    /// Count every directory entry, including interrupted temporary publications.
    pub fn with_checkpoint_limits(
        mut self,
        entries: usize,
        metadata: crate::ReclamationLimits,
    ) -> Result<Self> {
        if entries == 0 || entries > 65_536 {
            return Err(Error::InvalidConfiguration(
                "checkpoint entries must be in 1..=65536".into(),
            ));
        }
        metadata.validate()?;
        self.checkpoint_entries = entries;
        self.checkpoint_budget = metadata;
        Ok(self)
    }

    pub(super) fn checkpoint_entry_count(
        &self,
        budget: &mut crate::reclamation::ReclamationBudget,
    ) -> Result<usize> {
        let mut count = 0usize;
        for entry in self.root.join(CHECKPOINT_DIRECTORY).entries()? {
            budget.entry()?;
            entry?;
            count += 1;
            if count > self.checkpoint_entries {
                return Err(Error::CheckpointLimit {
                    limit: self.checkpoint_entries,
                    observed: count,
                });
            }
        }
        Ok(count)
    }

    pub fn checkpoint_ownership(&self) -> Result<(u64, u64)> {
        let _gate = self
            .checkpoint_gate
            .lock()
            .map_err(|_| Error::PoisonedWriter)?;
        let mut budget = crate::reclamation::ReclamationBudget::new(self.checkpoint_budget);
        Ok((
            self.checkpoint_entry_count(&mut budget)? as u64,
            self.checkpoint_entries as u64,
        ))
    }

    pub fn checkpoint(
        &self,
        name: &str,
        manifest_digest: &str,
        created_at: u64,
    ) -> Result<Checkpoint> {
        let _gate = self
            .checkpoint_gate
            .lock()
            .map_err(|_| Error::PoisonedWriter)?;
        self.ensure_publication_ready()?;
        let mut budget = crate::reclamation::ReclamationBudget::new(self.checkpoint_budget);
        let manifest = self.load_bounded(manifest_digest, &mut budget)?;
        let checkpoint = Checkpoint::new(name, &manifest, created_at)?;
        let directory = self.root.join(CHECKPOINT_DIRECTORY);
        let path = directory.join(format!("{name}.json"));
        let bytes = serde_json::to_vec(&checkpoint)?;
        if path.try_exists()? {
            let existing: Checkpoint = serde_json::from_slice(&budget.read_metadata(&path)?)?;
            existing.validate()?;
            if existing == checkpoint {
                self.publication_result(directory.sync_directory().map_err(Error::Io))?;
                return Ok(existing);
            }
            return Err(Error::InvalidManifest(format!(
                "checkpoint {name:?} already pins another manifest or instant"
            )));
        }
        let count = self.checkpoint_entry_count(&mut budget)?;
        if count >= self.checkpoint_entries {
            return Err(Error::CheckpointLimit {
                limit: self.checkpoint_entries,
                observed: count + 1,
            });
        }
        let temporary = directory.join(format!(
            ".{name}.{}.{}.tmp",
            self.temporary_nonce,
            POINTER_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed)
        ));
        self.publication_result(persist_rename(&directory, &temporary, &path, &bytes))?;
        Ok(checkpoint)
    }

    pub fn checkpoints(&self) -> Result<Vec<Checkpoint>> {
        let mut budget = crate::reclamation::ReclamationBudget::new(self.checkpoint_budget);
        self.checkpoints_inner(&mut budget)
    }

    pub(crate) fn checkpoints_bounded(
        &self,
        budget: &mut crate::reclamation::ReclamationBudget,
    ) -> Result<Vec<Checkpoint>> {
        self.checkpoints_inner(budget)
    }

    pub(super) fn checkpoints_inner(
        &self,
        budget: &mut crate::reclamation::ReclamationBudget,
    ) -> Result<Vec<Checkpoint>> {
        let _gate = self
            .checkpoint_gate
            .lock()
            .map_err(|_| Error::PoisonedWriter)?;
        let directory = self.root.join(CHECKPOINT_DIRECTORY);
        let mut checkpoints = Vec::new();
        let mut entries = Vec::new();
        for entry in directory.entries()? {
            budget.entry()?;
            if entries.len() >= self.checkpoint_entries {
                return Err(Error::CheckpointLimit {
                    limit: self.checkpoint_entries,
                    observed: entries.len() + 1,
                });
            }
            entries.push(entry?);
        }
        entries.sort_by(|left, right| left.relative_path().cmp(right.relative_path()));
        for entry in entries {
            if entry
                .relative_path()
                .extension()
                .and_then(|value| value.to_str())
                != Some("json")
            {
                continue;
            }
            let bytes = budget.read_metadata(&entry)?;
            let checkpoint: Checkpoint = serde_json::from_slice(&bytes)?;
            checkpoint.validate()?;
            let manifest = self.load_bounded(&checkpoint.manifest, budget)?;
            if manifest.generation != checkpoint.generation {
                return Err(Error::InvalidManifest(format!(
                    "checkpoint {:?} generation differs from its manifest",
                    checkpoint.name
                )));
            }
            checkpoints.push(checkpoint);
        }
        checkpoints.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(checkpoints)
    }

    pub fn release_checkpoint(&self, name: &str) -> Result<bool> {
        let _gate = self
            .checkpoint_gate
            .lock()
            .map_err(|_| Error::PoisonedWriter)?;
        self.ensure_publication_ready()?;
        validate_checkpoint_name(name)?;
        let directory = self.root.join(CHECKPOINT_DIRECTORY);
        let path = directory.join(format!("{name}.json"));
        match path.remove_file() {
            Ok(()) => {
                self.publication_result(directory.sync_directory().map_err(Error::Io))?;
                Ok(true)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => self.publication_result(Err(error.into())),
        }
    }

    pub(super) fn persist_manifest(&self, manifest: &Manifest) -> Result<()> {
        let directory = self.root.join(MANIFEST_DIRECTORY);
        let path = directory.join(format!("{}.json", manifest.digest));
        let bytes = serde_json::to_vec(manifest)?;
        match path.metadata() {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(Error::InvalidManifest(
                        "existing manifest is not a regular non-symlink file".into(),
                    ));
                }
                if path.read()? != bytes {
                    return Err(Error::InvalidManifest(
                        "existing manifest identity has different bytes".into(),
                    ));
                }
                directory.sync_directory()?;
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(Error::io(
                    "validating manifest destination",
                    path.diagnostic_path(),
                    error,
                ))
            }
        }
        let temporary = directory.join(format!(
            ".{}.{}.{}.tmp",
            manifest.digest,
            self.temporary_nonce,
            POINTER_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed)
        ));
        persist_rename(&directory, &temporary, &path, &bytes)
    }

    pub(super) fn publish_pointer(&self, pointer: &CurrentPointer) -> Result<()> {
        let path = self.root.join(CURRENT_FILE);
        let temporary = self.root.join(format!(
            ".CURRENT.{}.{}.tmp",
            self.temporary_nonce,
            POINTER_TEMPORARY_ID.fetch_add(1, Ordering::Relaxed)
        ));
        persist_rename(&self.root, &temporary, &path, &serde_json::to_vec(pointer)?)
    }
}

pub(super) fn validate_checkpoint_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 128
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(Error::InvalidManifest(format!(
            "invalid checkpoint name {name:?}"
        )));
    }
    Ok(())
}

pub(super) fn persist_rename(
    _directory: &AnchoredPath,
    temporary: &AnchoredPath,
    target: &AnchoredPath,
    bytes: &[u8],
) -> Result<()> {
    temporary.admit_disk(bytes.len() as u64, true)?;
    crate::filesystem::persist_rename(temporary, target, bytes).map_err(Error::Io)
}
