//! Filesystem authority retained independently of diagnostic pathnames.
//!
//! Each traversal opens one ordinary directory component without following
//! links, then checks its native mount identity. Effects use the retained
//! parent handle. A diagnostic pathname is never accepted by these methods.

use cap_fs_ext::OpenOptionsFollowExt;
use cap_primitives::fs as cap;
use std::ffi::OsString;
use std::fs::{File, Metadata};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

#[path = "filesystem/io.rs"]
pub(crate) mod file_io;
mod native;
pub(crate) use native::FileIdentity;
pub(crate) mod space;

#[cfg(test)]
use crate::fault_io::{Kind, Ticket};

#[derive(Debug)]
struct Root {
    recovery_reserve_bytes: u64,
    file: File,
    mount: native::MountIdentity,
    // Windows write-through publication uses native names. Retaining every
    // ancestor without delete sharing prevents those names being redirected.
    _ancestors: Vec<File>,
    _parent: Option<Arc<Root>>,
}

/// A relative name whose authority is an open root, not its displayed path.
/// Deliberately does not implement `AsRef<Path>` or dereference to `Path`.
#[derive(Clone, Debug)]
pub struct AnchoredPath {
    root: Arc<Root>,
    relative: PathBuf,
    diagnostic: PathBuf,
}

impl AnchoredPath {
    /// Establishes the initial authority. Callers must bind their authorization
    /// and inventory to this handle before performing durable effects.
    pub fn open_root(path: &Path) -> io::Result<Self> {
        let requested = std::path::absolute(path)?;
        // Resolve host-owned parent aliases (for example macOS /var), but
        // never follow the directory leaf being admitted as the authority.
        let absolute = match (requested.parent(), requested.file_name()) {
            (Some(parent), Some(name)) => std::fs::canonicalize(parent)?.join(name),
            _ => requested,
        };
        Self::open_exact_root(&absolute)
    }

    /// Opens an already normalized authorization root without following any
    /// component. Use this after binding a canonical project pathname.
    pub fn open_exact_root(path: &Path) -> io::Result<Self> {
        let absolute = std::path::absolute(path)?;
        let mut components = absolute.components();
        let mut prefix = PathBuf::new();
        for component in components.by_ref() {
            prefix.push(component);
            if component == Component::RootDir {
                break;
            }
        }
        let mut file = cap::open_ambient_dir(&prefix, cap_primitives::ambient_authority())?;
        let mut ancestors = Vec::new();
        for component in components {
            match component {
                Component::Normal(name) => {
                    let next = cap::open_dir_nofollow(&file, Path::new(name))?;
                    ancestors.push(file);
                    file = next;
                    native::validate_directory(&file)?;
                }
                _ => return Err(invalid_path()),
            }
        }
        native::validate_directory(&file)?;
        native::validate_root(&file)?;
        let mount = native::mount_identity(&file)?;
        Ok(Self {
            root: Arc::new(Root {
                file,
                mount,
                recovery_reserve_bytes: 0,
                _ancestors: ancestors,
                _parent: None,
            }),
            relative: PathBuf::new(),
            diagnostic: absolute,
        })
    }

    pub(crate) fn with_recovery_reserve(&self, bytes: u64) -> io::Result<Self> {
        Ok(Self {
            root: Arc::new(Root {
                recovery_reserve_bytes: bytes,
                file: self.root.file.try_clone()?,
                mount: self.root.mount.clone(),
                _ancestors: Vec::new(),
                _parent: Some(self.root.clone()),
            }),
            relative: self.relative.clone(),
            diagnostic: self.diagnostic.clone(),
        })
    }

    pub(crate) fn recovery_reserve(&self) -> u64 {
        self.root.recovery_reserve_bytes
    }

    pub(crate) fn admit_disk(&self, bytes: u64, new_file: bool) -> crate::Result<()> {
        space::admit(&self.root.file, self.recovery_reserve(), bytes, new_file)
    }

    pub fn join(&self, relative: impl AsRef<Path>) -> Self {
        Self {
            root: Arc::clone(&self.root),
            relative: self.relative.join(relative.as_ref()),
            diagnostic: self.diagnostic.join(relative),
        }
    }

    /// Establishes authority for a leaf, which may not yet exist. Subsequent
    /// operations retain its existing parent rather than reopening that name.
    pub fn from_path(path: &Path) -> io::Result<Self> {
        let absolute = std::path::absolute(path)?;
        let mut parent = absolute.parent().ok_or_else(invalid_path)?;
        let mut names = vec![absolute.file_name().ok_or_else(invalid_path)?.to_owned()];
        loop {
            match std::fs::symlink_metadata(parent) {
                Ok(_) => break,
                Err(error) if error.kind() == io::ErrorKind::NotFound && names.len() < 1024 => {
                    names.push(parent.file_name().ok_or_else(invalid_path)?.to_owned());
                    parent = parent.parent().ok_or_else(invalid_path)?;
                }
                Err(error) => return Err(error),
            }
        }
        let mut path = Self::open_root(parent)?;
        for name in names.into_iter().rev() {
            path = path.join(name);
        }
        Ok(path)
    }

    /// Diagnostics only; never pass this name back to ambient filesystem I/O.
    pub fn diagnostic_path(&self) -> &Path {
        &self.diagnostic
    }

    pub fn display(&self) -> std::path::Display<'_> {
        self.diagnostic.display()
    }

    pub fn relative_path(&self) -> &Path {
        &self.relative
    }

    /// Retains this exact directory as a new root without resolving its
    /// diagnostic pathname. The original mount admission remains in force.
    pub fn pin_directory(&self) -> io::Result<Self> {
        self.pin_directory_checked(|_| Ok(()))
    }

    pub fn pin_directory_checked(
        &self,
        validate: impl FnOnce(&File) -> io::Result<()>,
    ) -> io::Result<Self> {
        let (file, ancestors) = self.directory_chain()?;
        validate(&file)?;
        Ok(Self {
            root: Arc::new(Root {
                file,
                mount: self.root.mount.clone(),
                _ancestors: ancestors,
                recovery_reserve_bytes: self.root.recovery_reserve_bytes,
                _parent: Some(self.root.clone()),
            }),
            relative: PathBuf::new(),
            diagnostic: self.diagnostic.clone(),
        })
    }

    /// Native policy helpers receive a retained parent and one validated name.
    pub fn with_parent<T>(
        &self,
        operation: impl FnOnce(&File, &std::ffi::OsStr) -> io::Result<T>,
    ) -> io::Result<T> {
        let (parent, name, _ancestors) = self.parent_handle()?;
        operation(&parent, &name)
    }

    /// Admits the directory handle returned by an atomic native create/open
    /// helper without resolving its diagnostic name a second time.
    pub fn open_directory_with(
        &self,
        open: impl FnOnce(&File, &std::ffi::OsStr) -> io::Result<File>,
    ) -> io::Result<Self> {
        let (parent, name, mut ancestors) = self.parent_handle()?;
        let file = open(&parent, &name)?;
        ancestors.push(parent);
        self.admit_directory(&file)?;
        Ok(Self {
            root: Arc::new(Root {
                file,
                mount: self.root.mount.clone(),
                _ancestors: ancestors,
                recovery_reserve_bytes: self.root.recovery_reserve_bytes,
                _parent: Some(self.root.clone()),
            }),
            relative: PathBuf::new(),
            diagnostic: self.diagnostic.clone(),
        })
    }

    pub fn open_file_with(
        &self,
        open: impl FnOnce(&File, &std::ffi::OsStr) -> io::Result<File>,
    ) -> io::Result<File> {
        let file = self.with_parent(open)?;
        self.admit_mount(&file)?;
        self.validate_file(&file)?;
        Ok(file)
    }

    fn validate_file(&self, file: &File) -> io::Result<()> {
        native::validate_file(file).map_err(|error| {
            // Preserve native OS error identities. Only our own admission
            // denial gains bounded relative context; never emit file contents.
            if error.kind() != io::ErrorKind::PermissionDenied || error.raw_os_error().is_some() {
                return error;
            }
            let label: String = self.relative.to_string_lossy().chars().take(256).collect();
            io::Error::new(
                error.kind(),
                format!("file admission at {label:?}: {error}"),
            )
        })
    }

    pub fn parent(&self) -> Option<Self> {
        Some(Self {
            root: Arc::clone(&self.root),
            relative: self.relative.parent()?.to_owned(),
            diagnostic: self.diagnostic.parent()?.to_owned(),
        })
    }

    fn components(&self) -> io::Result<Vec<OsString>> {
        self.relative
            .components()
            .map(|component| match component {
                Component::Normal(name) if native::valid_component(name) => Ok(name.to_owned()),
                _ => Err(invalid_path()),
            })
            .collect()
    }

    fn directory(&self) -> io::Result<File> {
        self.directory_chain().map(|(file, _)| file)
    }

    fn directory_chain(&self) -> io::Result<(File, Vec<File>)> {
        let mut file = self.root.file.try_clone()?;
        let mut ancestors = Vec::new();
        for name in self.components()? {
            let next = cap::open_dir_nofollow(&file, Path::new(&name))?;
            self.admit_directory(&next)?;
            ancestors.push(file);
            file = next;
        }
        Ok((file, ancestors))
    }

    fn admit_directory(&self, file: &File) -> io::Result<()> {
        native::validate_directory(file)?;
        self.admit_mount(file)
    }

    fn admit_mount(&self, file: &File) -> io::Result<()> {
        if native::mount_identity(file)? != self.root.mount {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "filesystem mount crossing denied",
            ));
        }
        Ok(())
    }

    fn parent_handle(&self) -> io::Result<(File, OsString, Vec<File>)> {
        let components = self.components()?;
        let (name, parents) = components.split_last().ok_or_else(invalid_path)?;
        let mut file = self.root.file.try_clone()?;
        let mut ancestors = Vec::new();
        for parent in parents {
            let next = cap::open_dir_nofollow(&file, Path::new(parent))?;
            self.admit_directory(&next)?;
            ancestors.push(file);
            file = next;
        }
        Ok((file, name.clone(), ancestors))
    }

    pub fn create_dir(&self) -> io::Result<()> {
        let (parent, name, _ancestors) = self.parent_handle()?;
        let mut builder = cap::DirBuilder::new();
        #[cfg(unix)]
        {
            use cap::DirBuilderExt;
            builder.mode(0o700);
        }
        #[cfg(not(unix))]
        let _ = &mut builder;
        cap::create_dir(&parent, Path::new(&name), builder.options())?;
        native::sync_directory(&parent)
    }

    pub fn create_dir_all(&self) -> io::Result<()> {
        let components = self.components()?;
        let mut diagnostic = self.diagnostic.clone();
        for _ in &components {
            diagnostic.pop();
        }
        let mut current = Self {
            root: Arc::clone(&self.root),
            relative: PathBuf::new(),
            diagnostic,
        };
        for name in components {
            current = current.join(name);
            match current.create_dir() {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    current.directory()?;
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    pub fn metadata(&self) -> io::Result<Metadata> {
        if self.relative.as_os_str().is_empty() {
            return self.root.file.metadata();
        }
        let (parent, name, _ancestors) = self.parent_handle()?;
        let mut options = cap::OpenOptions::new();
        options.read(true).follow(cap::FollowSymlinks::No);
        native::metadata_options(&mut options);
        let file = cap::open(&parent, Path::new(&name), &options)?;
        self.admit_mount(&file)?;
        native::validate_object(&file)?;
        file.metadata()
    }

    pub fn try_exists(&self) -> io::Result<bool> {
        match self.metadata() {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    pub fn with_extension(&self, extension: impl AsRef<std::ffi::OsStr>) -> Self {
        Self {
            root: self.root.clone(),
            relative: self.relative.with_extension(extension.as_ref()),
            diagnostic: self.diagnostic.with_extension(extension.as_ref()),
        }
    }

    pub fn read(&self) -> io::Result<Vec<u8>> {
        #[cfg(test)]
        let ticket = Ticket::begin(Kind::Read, self.diagnostic_path())?;
        let mut bytes = Vec::new();
        self.open_read()?.read_to_end(&mut bytes)?;
        #[cfg(test)]
        ticket.done()?;
        Ok(bytes)
    }

    pub fn open_read(&self) -> io::Result<File> {
        OpenOptions::new().read(true).open(self)
    }

    pub fn read_dir(&self) -> io::Result<Vec<Self>> {
        let mut entries = self.entries()?.collect::<io::Result<Vec<_>>>()?;
        entries.sort_by(|left, right| left.relative.cmp(&right.relative));
        Ok(entries)
    }

    /// Streaming enumeration lets callers enforce admission bounds before
    /// retaining names, including when an untrusted directory is very large.
    pub fn entries(&self) -> io::Result<impl Iterator<Item = io::Result<Self>>> {
        let entries = cap::read_base_dir(&self.directory()?)?;
        let directory = self.clone();
        Ok(entries.map(move |entry| Ok(directory.join(entry?.file_name()))))
    }

    pub fn sync_directory(&self) -> io::Result<()> {
        let directory = self.directory()?;
        self.sync_directory_handle(&directory)
    }

    fn sync_directory_handle(&self, file: &File) -> io::Result<()> {
        #[cfg(test)]
        let ticket = Ticket::begin(Kind::DirectorySync, self.diagnostic_path())?;
        let result = native::sync_directory(file);
        #[cfg(all(feature = "crash-test", not(test)))]
        crate::crash_test::observe_native_error("directory_sync", self.diagnostic_path(), &result);
        result?;
        #[cfg(test)]
        ticket.directory_synced()?;
        Ok(())
    }

    pub fn remove_file(&self) -> io::Result<()> {
        let (parent, name, _ancestors) = self.parent_handle()?;
        // unlinkat semantics remove this directory entry, never its link target.
        #[cfg(test)]
        let ticket = Ticket::begin(Kind::Remove, self.diagnostic_path())?;
        #[cfg(all(feature = "crash-test", not(test)))]
        crate::crash_test::before_io("remove", self.diagnostic_path());
        cap::remove_file(&parent, Path::new(&name))?;
        #[cfg(all(feature = "crash-test", not(test)))]
        crate::crash_test::after_io("remove", self.diagnostic_path());
        #[cfg(test)]
        ticket.removed()?;
        self.parent()
            .ok_or_else(invalid_path)?
            .sync_directory_handle(&parent)
    }

    pub fn rename_new_to(&self, target: &Self) -> io::Result<()> {
        let (source_parent, source_name, _source_ancestors) = self.parent_handle()?;
        let (target_parent, target_name, _target_ancestors) = target.parent_handle()?;
        #[cfg(test)]
        let ticket = Ticket::begin(Kind::Rename, self.diagnostic_path())?;
        native::rename_new(
            &source_parent,
            Path::new(&source_name),
            &target_parent,
            Path::new(&target_name),
        )?;
        #[cfg(test)]
        ticket.renamed(target.diagnostic_path())?;
        self.parent()
            .ok_or_else(invalid_path)?
            .sync_directory_handle(&source_parent)?;
        target
            .parent()
            .ok_or_else(invalid_path)?
            .sync_directory_handle(&target_parent)
    }

    pub fn rename_to(&self, target: &Self) -> io::Result<()> {
        let (source_parent, source_name, _source_ancestors) = self.parent_handle()?;
        let (target_parent, target_name, _target_ancestors) = target.parent_handle()?;
        #[cfg(test)]
        let ticket = Ticket::begin(Kind::Rename, self.diagnostic_path())?;
        #[cfg(all(feature = "crash-test", not(test)))]
        crate::crash_test::before_io("rename", self.diagnostic_path());
        let result = native::rename(
            &source_parent,
            Path::new(&source_name),
            &target_parent,
            Path::new(&target_name),
        );
        #[cfg(all(feature = "crash-test", not(test)))]
        crate::crash_test::observe_native_error("rename", self.diagnostic_path(), &result);
        result?;
        #[cfg(all(feature = "crash-test", not(test)))]
        crate::crash_test::after_io("rename", self.diagnostic_path());
        #[cfg(test)]
        ticket.renamed(target.diagnostic_path())?;
        self.parent()
            .ok_or_else(invalid_path)?
            .sync_directory_handle(&source_parent)?;
        target
            .parent()
            .ok_or_else(invalid_path)?
            .sync_directory_handle(&target_parent)
    }
}

/// Options that open only an anchored ordinary file. Truncation occurs after
/// native object and mount admission on the very same handle.
#[derive(Default)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    create: bool,
    create_new: bool,
    truncate: bool,
    append: bool,
}

impl OpenOptions {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn read(&mut self, value: bool) -> &mut Self {
        self.read = value;
        self
    }
    pub fn write(&mut self, value: bool) -> &mut Self {
        self.write = value;
        self
    }
    pub fn create(&mut self, value: bool) -> &mut Self {
        self.create = value;
        self
    }
    pub fn create_new(&mut self, value: bool) -> &mut Self {
        self.create_new = value;
        self
    }
    pub fn truncate(&mut self, value: bool) -> &mut Self {
        self.truncate = value;
        self
    }
    pub fn append(&mut self, value: bool) -> &mut Self {
        self.append = value;
        self
    }

    pub fn open(&self, path: &AnchoredPath) -> io::Result<File> {
        let (parent, name, _ancestors) = path.parent_handle()?;
        let mut options = cap::OpenOptions::new();
        options
            .read(self.read)
            .write(self.write)
            .append(self.append)
            .create(self.create)
            .create_new(self.create_new)
            .follow(cap::FollowSymlinks::No);
        native::file_options(&mut options);
        #[cfg(test)]
        let ticket = Ticket::begin(Kind::Open, path.diagnostic_path())?;
        #[cfg(all(feature = "crash-test", not(test)))]
        if self.create || self.create_new {
            crate::crash_test::before_io("create", path.diagnostic_path());
        }
        let result = cap::open(&parent, Path::new(&name), &options);
        #[cfg(all(feature = "crash-test", not(test)))]
        crate::crash_test::observe_native_error("open", path.diagnostic_path(), &result);
        let file = result?;
        path.admit_mount(&file)?;
        path.validate_file(&file)?;
        if self.truncate {
            file.set_len(0)?;
        }
        #[cfg(test)]
        ticket.opened()?;
        Ok(file)
    }
}

/// Durable create-new publication; the caller owns its declared temporary name.
pub fn persist_rename(
    temporary: &AnchoredPath,
    target: &AnchoredPath,
    bytes: &[u8],
) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temporary)?;
    let result = (|| {
        file_io::write_all(&mut file, temporary.diagnostic_path(), bytes)?;
        file_io::sync_all(&file, temporary.diagnostic_path())?;
        temporary.rename_to(target)
    })();
    drop(file);
    if result.is_err() {
        let _ = temporary.remove_file();
    }
    result
}

/// Obtains an independent read cursor only if native identity still matches
/// the admitted file. No content is read before that identity check.
pub fn reopen_read(file: &File) -> io::Result<File> {
    let mut options = cap::OpenOptions::new();
    options.read(true).follow(cap::FollowSymlinks::No);
    native::file_options(&mut options);
    let reopened = cap::reopen(file, &options)?;
    native::validate_file(&reopened)?;
    Ok(reopened)
}

fn invalid_path() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "anchored filesystem requires ordinary relative components",
    )
}

/// Explicit admission at a public filesystem boundary. Already anchored inputs
/// retain their authority; ambient paths are captured once before any effect.
pub trait AnchorInput {
    fn anchor(&self) -> io::Result<AnchoredPath>;
}
impl AnchorInput for AnchoredPath {
    fn anchor(&self) -> io::Result<AnchoredPath> {
        Ok(self.clone())
    }
}
impl AnchorInput for Path {
    fn anchor(&self) -> io::Result<AnchoredPath> {
        AnchoredPath::from_path(self)
    }
}
impl AnchorInput for PathBuf {
    fn anchor(&self) -> io::Result<AnchoredPath> {
        self.as_path().anchor()
    }
}

impl<T: AnchorInput + ?Sized> AnchorInput for &T {
    fn anchor(&self) -> io::Result<AnchoredPath> {
        (**self).anchor()
    }
}
