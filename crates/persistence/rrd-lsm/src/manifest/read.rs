//! Reading the current manifest.

use super::*;

impl ManifestStore {
    pub fn current(&self) -> Result<Option<(CurrentPointer, Manifest)>> {
        self.current_using(|path| Ok(path.read()?))
    }

    pub(crate) fn current_bounded(
        &self,
        budget: &mut crate::recovery_budget::RecoveryBudget,
    ) -> Result<Option<(CurrentPointer, Manifest)>> {
        self.current_using(|path| budget.metadata(path))
    }

    pub(super) fn current_using(
        &self,
        mut read: impl FnMut(&AnchoredPath) -> Result<Vec<u8>>,
    ) -> Result<Option<(CurrentPointer, Manifest)>> {
        let path = self.root.join(CURRENT_FILE);
        match path.metadata() {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(Error::InvalidManifest(
                    "CURRENT is not a regular non-symlink file".into(),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(Error::io(
                    "validating CURRENT",
                    path.diagnostic_path(),
                    error,
                ))
            }
        }
        let bytes = read(&path)?;
        let pointer: CurrentPointer = serde_json::from_slice(&bytes)?;
        pointer.validate()?;
        let manifest = self.load_using(&pointer.manifest, read)?;
        if manifest.generation != pointer.generation {
            return Err(Error::InvalidManifest(
                "CURRENT generation differs from its manifest".into(),
            ));
        }
        Ok(Some((pointer, manifest)))
    }

    pub fn load(&self, digest: &str) -> Result<Manifest> {
        self.load_using(digest, |path| Ok(path.read()?))
    }

    pub(crate) fn load_bounded(
        &self,
        digest: &str,
        budget: &mut crate::reclamation::ReclamationBudget,
    ) -> Result<Manifest> {
        self.load_using(digest, |path| budget.read_metadata(path))
    }

    pub(super) fn load_using(
        &self,
        digest: &str,
        mut read: impl FnMut(&AnchoredPath) -> Result<Vec<u8>>,
    ) -> Result<Manifest> {
        if !is_sha256(digest) {
            return Err(Error::InvalidManifest(
                "manifest lookup requires a SHA-256 identity".into(),
            ));
        }
        let path = self
            .root
            .join(MANIFEST_DIRECTORY)
            .join(format!("{digest}.json"));
        let metadata = path.metadata().map_err(|error| {
            Error::io("validating manifest file", path.diagnostic_path(), error)
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(Error::InvalidManifest(
                "manifest is not a regular non-symlink file".into(),
            ));
        }
        let bytes = read(&path)?;
        let manifest: Manifest = serde_json::from_slice(&bytes)?;
        manifest.validate()?;
        if manifest.digest != digest {
            return Err(Error::InvalidManifest(
                "manifest filename differs from its content identity".into(),
            ));
        }
        Ok(manifest)
    }
}
