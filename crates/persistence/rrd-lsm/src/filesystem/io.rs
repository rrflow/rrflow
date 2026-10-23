//! Native file calls shared by WAL and immutable publication. Fault scheduling
//! and byte observation exist only in this crate's unit-test build.

use std::fs::File;
use std::io::{self, IoSlice, Read, Write};
use std::path::Path;

#[cfg(test)]
use crate::fault_io::{Kind, Ticket};
#[cfg(test)]
use std::io::Seek;

pub(crate) fn write_vectored(
    file: &mut File,
    _path: &Path,
    buffers: &[IoSlice<'_>],
) -> io::Result<usize> {
    #[cfg(test)]
    let ticket = Ticket::begin(Kind::Write, _path)?;
    #[cfg(test)]
    let offset = file.stream_position()?;
    #[cfg(test)]
    let limited;
    #[cfg(test)]
    let buffers = if let Some(limit) = ticket.write_limit()? {
        limited = buffers
            .iter()
            .flat_map(|buffer| buffer.iter().copied())
            .take(limit)
            .collect::<Vec<_>>();
        &[IoSlice::new(&limited)][..]
    } else {
        buffers
    };
    let result = file.write_vectored(buffers);
    #[cfg(all(feature = "crash-test", not(test)))]
    crate::crash_test::observe_native_error("write", _path, &result);
    let written = result?;
    #[cfg(test)]
    ticket.written(
        offset,
        &buffers
            .iter()
            .flat_map(|buffer| buffer.iter().copied())
            .take(written)
            .collect::<Vec<_>>(),
    )?;
    Ok(written)
}

pub(crate) fn write_all(file: &mut File, path: &Path, mut bytes: &[u8]) -> io::Result<()> {
    while !bytes.is_empty() {
        match write_vectored(file, path, &[IoSlice::new(bytes)]) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(written) => bytes = &bytes[written..],
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

pub(crate) fn sync_data(file: &File, _path: &Path) -> io::Result<()> {
    #[cfg(test)]
    let ticket = Ticket::begin(Kind::SyncData, _path)?;
    let result = file.sync_data();
    #[cfg(all(feature = "crash-test", not(test)))]
    crate::crash_test::observe_native_error("sync_data", _path, &result);
    result?;
    #[cfg(test)]
    ticket.synced()?;
    Ok(())
}

pub(crate) fn sync_all(file: &File, _path: &Path) -> io::Result<()> {
    #[cfg(test)]
    let ticket = Ticket::begin(Kind::SyncAll, _path)?;
    let result = file.sync_all();
    #[cfg(all(feature = "crash-test", not(test)))]
    crate::crash_test::observe_native_error("sync_all", _path, &result);
    result?;
    #[cfg(test)]
    ticket.synced()?;
    Ok(())
}

pub(crate) fn read_exact(file: &mut File, bytes: &mut [u8]) -> io::Result<()> {
    #[cfg(test)]
    let ticket = Ticket::begin(Kind::Read, Path::new(""))?;
    file.read_exact(bytes)?;
    #[cfg(test)]
    ticket.done()?;
    Ok(())
}

/// Read at most the caller's admitted bytes while preserving the native
/// fault-observation boundary used by ordinary metadata reads.
pub(crate) fn read_to_limit(file: File, _path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    #[cfg(test)]
    let ticket = Ticket::begin(Kind::Read, _path)?;
    let mut bytes = Vec::new();
    file.take(limit).read_to_end(&mut bytes)?;
    #[cfg(test)]
    ticket.done()?;
    Ok(bytes)
}
