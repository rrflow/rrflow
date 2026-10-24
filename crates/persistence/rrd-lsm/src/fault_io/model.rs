//! Deliberately simple device oracle: file sync persists bytes, directory sync
//! persists names. Recovery uses only stable names/bytes. This is an abstract
//! I/O model, not a claim about any OS, device cache, torn sector or power cut.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MAX_FILES: usize = 128;
const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone)]
struct Object {
    live: Vec<u8>,
    stable: Vec<u8>,
}

pub(super) struct Device {
    root: PathBuf,
    live: BTreeMap<PathBuf, usize>,
    stable: BTreeMap<PathBuf, usize>,
    objects: Vec<Object>,
}

impl Device {
    pub(super) fn seed(root: &Path) -> Self {
        let root = root.canonicalize().unwrap();
        let mut device = Self {
            root,
            live: BTreeMap::new(),
            stable: BTreeMap::new(),
            objects: Vec::new(),
        };
        for (path, bytes) in inventory(&device.root) {
            let id = device.objects.len();
            device.live.insert(path.clone(), id);
            device.stable.insert(path, id);
            device.objects.push(Object {
                live: bytes.clone(),
                stable: bytes,
            });
        }
        device.check_bounds();
        device
    }

    fn key(&self, path: &Path) -> PathBuf {
        path.strip_prefix(&self.root)
            .expect("I/O outside owned fixture")
            .to_owned()
    }

    fn check_bounds(&self) {
        assert!(self.objects.len() <= MAX_FILES, "model file bound");
        assert!(
            self.objects
                .iter()
                .map(|object| object.live.len() + object.stable.len())
                .sum::<usize>()
                <= MAX_BYTES,
            "model byte bound"
        );
    }

    pub(super) fn opened(&mut self, path: &Path) {
        let key = self.key(path);
        if !self.live.contains_key(&key) {
            self.live.insert(key, self.objects.len());
            self.objects.push(Object {
                live: Vec::new(),
                stable: Vec::new(),
            });
        }
        self.check_bounds();
    }

    pub(super) fn write(&mut self, path: &Path, offset: u64, bytes: &[u8]) {
        let offset = usize::try_from(offset).unwrap();
        let end = offset.checked_add(bytes.len()).unwrap();
        assert!(end <= MAX_BYTES);
        let id = self.live[&self.key(path)];
        let data = &mut self.objects[id].live;
        data.resize(data.len().max(end), 0);
        data[offset..end].copy_from_slice(bytes);
        self.check_bounds();
    }

    pub(super) fn sync_file(&mut self, path: &Path) {
        let id = self.live[&self.key(path)];
        self.objects[id].stable = self.objects[id].live.clone();
        self.check_bounds();
    }

    pub(super) fn sync_directory(&mut self, path: &Path) {
        let directory = self.key(path);
        self.stable
            .retain(|name, _| name.parent() != Some(directory.as_path()));
        self.stable.extend(
            self.live
                .iter()
                .filter(|(name, _)| name.parent() == Some(directory.as_path()))
                .map(|(name, id)| (name.clone(), *id)),
        );
    }

    pub(super) fn rename(&mut self, source: &Path, target: &Path) {
        let id = self
            .live
            .remove(&self.key(source))
            .expect("modeled rename source");
        self.live.insert(self.key(target), id);
    }

    pub(super) fn remove(&mut self, path: &Path) {
        self.live.remove(&self.key(path));
    }

    pub(super) fn crash_image(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        self.stable
            .iter()
            .map(|(name, id)| (name.clone(), self.objects[*id].stable.clone()))
            .collect()
    }

    /// Called only after every database/file/mapping/anchor has been dropped.
    /// Mutates regular files in this test's own temporary root, never a host volume.
    pub(super) fn lose_volatile(&self) {
        for name in inventory(&self.root).keys() {
            std::fs::remove_file(self.root.join(name)).unwrap();
        }
        for (name, bytes) in self.crash_image() {
            std::fs::write(self.root.join(name), bytes).unwrap();
        }
    }
}

pub(super) fn inventory(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let kind = entry.file_type().unwrap();
            assert!(!kind.is_symlink());
            if kind.is_dir() {
                visit(root, &entry.path(), files);
            } else {
                assert!(kind.is_file());
                // Windows denies reads through an exclusive MANIFEST.LOCK.
                // Its verified zero length is sufficient; no content is inferred.
                let bytes = if entry.metadata().unwrap().len() == 0 {
                    Vec::new()
                } else {
                    std::fs::read(entry.path()).unwrap()
                };
                files.insert(entry.path().strip_prefix(root).unwrap().to_owned(), bytes);
                assert!(files.len() <= MAX_FILES);
                assert!(files.values().map(Vec::len).sum::<usize>() <= MAX_BYTES);
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}
