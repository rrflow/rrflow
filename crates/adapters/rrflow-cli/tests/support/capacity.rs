//! Real native capacity corpus. Only the owned-image runner supplies admission.
use super::*;
use serde::Deserialize;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

#[path = "capacity_observation.rs"]
mod observation;

#[path = "capacity_metadata.rs"]
mod metadata;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FillBarrier {
    Interval,
    Capacity,
}

#[cfg(target_os = "macos")]
#[path = "capacity_apfs.rs"]
mod apfs;

// One wall-clock budget bounds a whole owned-volume fill: the data phase, its
// barriers, every joint settlement round and the metadata fill share the clock
// started in `Filler::fill`. The budget exists so a filesystem that never
// denies capacity still terminates the fixture; the entry and byte bounds cap
// the work itself. It is sized from hosted-runner evidence, not from a quiet
// machine: on the macOS image a near-full APFS volume creates and attributes a
// filler file in 15-40 ms, and a healthy fill needs 600-900 files before the
// first denial (candidate runs 42668299956: 876 files, 13.6 s of creates;
// 42656889121: 589 files, 22.6 s, expired). A 25 s budget failed five times in
// the 200 runs before 2026-11-09 with the volume still accepting creates.
pub(super) const FILL_BOUND: Duration = Duration::from_secs(120);

/// A clock that already exhausted `FILL_BOUND`, for bound tests.
pub(super) fn expired_fill_clock() -> Instant {
    Instant::now()
        .checked_sub(FILL_BOUND)
        .expect("process uptime exceeds the fill bound")
}

#[path = "capacity/cases.rs"]
mod cases;
#[path = "capacity/filler.rs"]
mod filler;
#[path = "capacity/filler_tests.rs"]
mod filler_tests;
#[path = "capacity/volume.rs"]
mod volume;

use filler::*;
use volume::*;

#[test]
fn apfs_wal_consumes_metadata_pressure_outside_authoritative_wal_directory() {
    cases::apfs_wal_consumes_metadata_pressure_outside_authoritative_wal_directory()
}

#[test]
#[ignore = "requires runner-owned ext4/APFS/NTFS image; never fill a developer volume"]
fn installed_native_capacity_recovers_acknowledged_history() {
    cases::installed_native_capacity_recovers_acknowledged_history()
}

#[test]
fn installed_capacity_wal_payload_is_observed_before_domain_commit() {
    cases::installed_capacity_wal_payload_is_observed_before_domain_commit()
}

#[test]
fn filler_allocation_sync_preserves_regular_file_contents() {
    filler::filler_allocation_sync_preserves_regular_file_contents()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn filler_host_sync_preserves_native_noncapacity_errors() {
    filler::filler_host_sync_preserves_native_noncapacity_errors()
}
