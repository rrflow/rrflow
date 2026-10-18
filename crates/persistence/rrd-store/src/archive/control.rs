//! Content-authenticated archive of one control-journal subsequence.
//!
//! An application-complete backup carries the estate's control state as the
//! ordered control-journal entries whose keys fall under a declared set of
//! families, not as materialized values. Readers such as the security audit
//! chain derive state from the journal's actions and order, so a restore must
//! replay the same transitions in the same order. The archive streams: memory
//! stays constant however long the journal is.
//!
//! Replacement bytes are framed raw after each entry's header instead of as a
//! JSON byte array, so the archive stays close to the size of the values it
//! carries.

use super::format::{
    archive_io, archive_read_error, hex, read_hashed, read_u16_hashed, read_u64_hashed,
    read_u64_raw,
};
use crate::control::{validate_control_key, ControlJournalEntry};
use crate::{Error, Result};
use rrd_core::digest::Sha256;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

pub(super) const MAGIC: &[u8; 8] = b"RRDCJA01";
pub const CONTROL_ARCHIVE_VERSION: u16 = 1;
const ENTRY_TAG: u8 = 1;
const FOOTER_TAG: u8 = 0xff;
/// A control value is bounded at one MiB by the control contract; the header
/// carries identities and digests only.
const MAX_CONTROL_VALUE_BYTES: usize = 1024 * 1024;
const MAX_ENTRY_HEADER_BYTES: usize = 64 * 1024;
const MAX_FAMILIES: usize = 256;

/// The two catalogue control families travel in the catalogue manifest, so a
/// control-journal scope may neither contain nor lie beneath them.
pub(crate) const CATALOGUE_CONTROL_FAMILIES: [&str; 2] = [
    "server/state/vector-collection-catalogue",
    "server/state/index-catalogue",
];

/// Inventory of one retained control-journal archive, bound into the backup
/// catalogue entry that references it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlArchiveInventory {
    pub format_version: u16,
    pub archive_sha256: String,
    pub source_control_sequence: u64,
    pub families: Vec<String>,
    pub entry_count: u64,
    pub payload_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_sequence: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_sequence: Option<u64>,
}

/// Everything of a journal entry except its replacement bytes, which follow
/// the header raw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryHeader {
    sequence: u64,
    key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    before_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    replacement_bytes: Option<u64>,
    at: u64,
    actor: String,
    action: String,
    request_id: String,
    operation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    previous_digest: Option<String>,
    digest: String,
}

impl EntryHeader {
    fn of(entry: &ControlJournalEntry) -> Self {
        Self {
            sequence: entry.sequence,
            key: entry.key.clone(),
            before_sha256: entry.before_sha256.clone(),
            replacement_bytes: entry
                .replacement
                .as_ref()
                .map(|replacement| replacement.len() as u64),
            at: entry.at,
            actor: entry.actor.clone(),
            action: entry.action.clone(),
            request_id: entry.request_id.clone(),
            operation_id: entry.operation_id.clone(),
            previous_digest: entry.previous_digest.clone(),
            digest: entry.digest.clone(),
        }
    }

    fn into_entry(self, replacement: Option<Vec<u8>>) -> ControlJournalEntry {
        ControlJournalEntry {
            sequence: self.sequence,
            key: self.key,
            before_sha256: self.before_sha256,
            replacement,
            at: self.at,
            actor: self.actor,
            action: self.action,
            request_id: self.request_id,
            operation_id: self.operation_id,
            previous_digest: self.previous_digest,
            digest: self.digest,
        }
    }
}

/// Validates one declared control-journal scope: canonical control keys,
/// strictly ascending, no family beneath another, and disjoint from the
/// catalogue families the catalogue manifest already carries.
pub fn validate_control_families(families: &[String]) -> Result<()> {
    if families.len() > MAX_FAMILIES {
        return Err(Error::Archive(
            "control-journal scope exceeds its v1 family bound".into(),
        ));
    }
    for (index, family) in families.iter().enumerate() {
        validate_control_key(family).map_err(|_| {
            Error::Archive(format!(
                "control-journal family is not a canonical control key: {family}"
            ))
        })?;
        if family.ends_with('/') {
            return Err(Error::Archive(format!(
                "control-journal family may not end with a separator: {family}"
            )));
        }
        if index > 0 && families[index - 1].as_str() >= family.as_str() {
            return Err(Error::Archive(
                "control-journal families are not uniquely ordered".into(),
            ));
        }
        for other in families.iter().take(index) {
            if control_family_covers(other, family) || control_family_covers(family, other) {
                return Err(Error::Archive(format!(
                    "control-journal families overlap: {other} and {family}"
                )));
            }
        }
        for catalogue in CATALOGUE_CONTROL_FAMILIES {
            if control_family_covers(catalogue, family) || control_family_covers(family, catalogue)
            {
                return Err(Error::Archive(format!(
                    "control-journal family overlaps the catalogue manifest scope: {family}"
                )));
            }
        }
    }
    Ok(())
}

/// A family covers its own key and every key beneath it as a path segment.
/// `server/state/estate` covers `server/state/estate/x` but not
/// `server/state/estate-recovery/x`.
pub fn control_family_covers(family: &str, key: &str) -> bool {
    key == family
        || (key.len() > family.len()
            && key.starts_with(family)
            && key.as_bytes()[family.len()] == b'/')
}

/// The family of `families` that covers `key`, if any.
pub fn control_family_of<'a>(families: &'a [String], key: &str) -> Option<&'a str> {
    families
        .iter()
        .map(String::as_str)
        .find(|family| control_family_covers(family, key))
}

#[derive(Default)]
struct ReplayState {
    entry_count: u64,
    payload_bytes: u64,
    first_sequence: Option<u64>,
    last_sequence: Option<u64>,
}

impl ReplayState {
    fn accept(
        &mut self,
        entry: &ControlJournalEntry,
        families: &[String],
        source_control_sequence: u64,
    ) -> Result<()> {
        if !entry.verify() {
            return Err(Error::Archive(format!(
                "control-journal archive entry {} fails its digest",
                entry.sequence
            )));
        }
        if entry.sequence == 0
            || entry.sequence > source_control_sequence
            || self
                .last_sequence
                .is_some_and(|last| entry.sequence <= last)
        {
            return Err(Error::Archive(
                "control-journal archive entries are not strictly ascending within the source watermark"
                    .into(),
            ));
        }
        validate_control_key(&entry.key)?;
        if control_family_of(families, &entry.key).is_none() {
            return Err(Error::Archive(format!(
                "control-journal archive entry {} lies outside the declared scope: {}",
                entry.sequence, entry.key
            )));
        }
        if entry
            .replacement
            .as_ref()
            .is_some_and(|replacement| replacement.len() > MAX_CONTROL_VALUE_BYTES)
        {
            return Err(Error::Archive(
                "control-journal archive value exceeds one MiB".into(),
            ));
        }
        self.entry_count = self.entry_count.checked_add(1).ok_or_else(|| {
            Error::Archive("control-journal archive entry counter overflow".into())
        })?;
        self.payload_bytes = self
            .payload_bytes
            .checked_add(
                entry
                    .replacement
                    .as_ref()
                    .map_or(0, |replacement| replacement.len() as u64),
            )
            .ok_or_else(|| {
                Error::Archive("control-journal archive payload counter overflow".into())
            })?;
        self.first_sequence.get_or_insert(entry.sequence);
        self.last_sequence = Some(entry.sequence);
        Ok(())
    }

    fn inventory(
        &self,
        archive_sha256: String,
        source_control_sequence: u64,
        families: &[String],
    ) -> ControlArchiveInventory {
        ControlArchiveInventory {
            format_version: CONTROL_ARCHIVE_VERSION,
            archive_sha256,
            source_control_sequence,
            families: families.to_vec(),
            entry_count: self.entry_count,
            payload_bytes: self.payload_bytes,
            first_sequence: self.first_sequence,
            last_sequence: self.last_sequence,
        }
    }
}

/// Streams one control-journal subsequence into a new archive file.
pub(crate) struct ControlArchiveWriter {
    file: File,
    digest: Sha256,
    source_control_sequence: u64,
    families: Vec<String>,
    state: ReplayState,
}

impl ControlArchiveWriter {
    pub fn create(path: &Path, source_control_sequence: u64, families: &[String]) -> Result<Self> {
        validate_control_families(families)?;
        let mut writer = Self {
            file: OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(archive_io)?,
            digest: Sha256::new(),
            source_control_sequence,
            families: families.to_vec(),
            state: ReplayState::default(),
        };
        writer.hashed(MAGIC)?;
        writer.hashed(&CONTROL_ARCHIVE_VERSION.to_be_bytes())?;
        writer.hashed(&source_control_sequence.to_be_bytes())?;
        writer.hashed(&(families.len() as u64).to_be_bytes())?;
        for family in families {
            writer.hashed(&(family.len() as u64).to_be_bytes())?;
            writer.hashed(family.as_bytes())?;
        }
        Ok(writer)
    }

    pub fn entry(&mut self, entry: &ControlJournalEntry) -> Result<()> {
        let header = serde_json::to_vec(&EntryHeader::of(entry))?;
        if header.len() > MAX_ENTRY_HEADER_BYTES {
            return Err(Error::Archive(
                "control-journal archive entry header exceeds its v1 bound".into(),
            ));
        }
        self.state
            .accept(entry, &self.families, self.source_control_sequence)?;
        self.hashed(&[ENTRY_TAG])?;
        self.hashed(&(header.len() as u64).to_be_bytes())?;
        self.hashed(&header)?;
        if let Some(replacement) = &entry.replacement {
            self.hashed(replacement)?;
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<ControlArchiveInventory> {
        let digest = self.digest.clone().finalize();
        self.file.write_all(&[FOOTER_TAG]).map_err(archive_io)?;
        for value in [
            self.state.entry_count,
            self.state.payload_bytes,
            self.state.first_sequence.unwrap_or_default(),
            self.state.last_sequence.unwrap_or_default(),
        ] {
            self.file
                .write_all(&value.to_be_bytes())
                .map_err(archive_io)?;
        }
        self.file.write_all(&digest).map_err(archive_io)?;
        self.file.sync_all().map_err(archive_io)?;
        Ok(self
            .state
            .inventory(hex(digest), self.source_control_sequence, &self.families))
    }

    fn hashed(&mut self, bytes: &[u8]) -> Result<()> {
        self.file.write_all(bytes).map_err(archive_io)?;
        self.digest.update(bytes);
        Ok(())
    }
}

/// Authenticates a control-journal archive without yielding its entries.
pub fn inspect_control_archive(path: &Path) -> Result<ControlArchiveInventory> {
    read_control_archive(path, |_| Ok(()))
}

/// Streams every entry of an authenticated control-journal archive in order.
/// The footer digest is checked after the last entry; a caller that must not
/// act on unauthenticated entries inspects the archive first.
pub fn read_control_archive(
    path: &Path,
    mut accept: impl FnMut(&ControlJournalEntry) -> Result<()>,
) -> Result<ControlArchiveInventory> {
    let mut file = File::open(path).map_err(archive_io)?;
    let mut digest = Sha256::new();
    let mut magic = [0u8; 8];
    read_hashed(&mut file, &mut digest, &mut magic)?;
    if &magic != MAGIC {
        return Err(Error::Archive(
            "control-journal archive magic does not match".into(),
        ));
    }
    let version = read_u16_hashed(&mut file, &mut digest)?;
    if version != CONTROL_ARCHIVE_VERSION {
        return Err(Error::Archive(format!(
            "unsupported control-journal archive version {version}"
        )));
    }
    let source_control_sequence = read_u64_hashed(&mut file, &mut digest)?;
    let family_count = read_u64_hashed(&mut file, &mut digest)?;
    if family_count > MAX_FAMILIES as u64 {
        return Err(Error::Archive(
            "control-journal archive declares too many families".into(),
        ));
    }
    let mut families = Vec::with_capacity(family_count as usize);
    for _ in 0..family_count {
        let length = read_u64_hashed(&mut file, &mut digest)?;
        if length > 256 {
            return Err(Error::Archive(
                "control-journal archive family name exceeds the control key bound".into(),
            ));
        }
        let mut bytes = vec![0; length as usize];
        read_hashed(&mut file, &mut digest, &mut bytes)?;
        families.push(String::from_utf8(bytes).map_err(|_| {
            Error::Archive("control-journal archive family name is not UTF-8".into())
        })?);
    }
    validate_control_families(&families)?;

    let mut state = ReplayState::default();
    loop {
        let mut tag = [0u8; 1];
        file.read_exact(&mut tag).map_err(archive_read_error)?;
        if tag[0] == FOOTER_TAG {
            break;
        }
        if tag[0] != ENTRY_TAG {
            return Err(Error::Archive(format!(
                "unknown control-journal archive record tag {}",
                tag[0]
            )));
        }
        digest.update(&tag);
        let header_length = read_u64_hashed(&mut file, &mut digest)?;
        let header_length = usize::try_from(header_length).map_err(|_| {
            Error::Archive("control-journal archive header length exceeds usize".into())
        })?;
        if header_length > MAX_ENTRY_HEADER_BYTES {
            return Err(Error::Archive(
                "control-journal archive entry header exceeds its v1 bound".into(),
            ));
        }
        let mut header = vec![0; header_length];
        read_hashed(&mut file, &mut digest, &mut header)?;
        let header: EntryHeader = serde_json::from_slice(&header)?;
        let replacement = match header.replacement_bytes {
            None => None,
            Some(length) => {
                let length = usize::try_from(length).map_err(|_| {
                    Error::Archive("control-journal archive value length exceeds usize".into())
                })?;
                if length > MAX_CONTROL_VALUE_BYTES {
                    return Err(Error::Archive(
                        "control-journal archive value exceeds one MiB".into(),
                    ));
                }
                let mut bytes = vec![0; length];
                read_hashed(&mut file, &mut digest, &mut bytes)?;
                Some(bytes)
            }
        };
        let entry = header.into_entry(replacement);
        state.accept(&entry, &families, source_control_sequence)?;
        accept(&entry)?;
    }

    let footer = [
        read_u64_raw(&mut file)?,
        read_u64_raw(&mut file)?,
        read_u64_raw(&mut file)?,
        read_u64_raw(&mut file)?,
    ];
    let mut declared_digest = [0u8; 32];
    file.read_exact(&mut declared_digest)
        .map_err(archive_read_error)?;
    let mut trailing = [0u8; 1];
    if file.read(&mut trailing).map_err(archive_io)? != 0 {
        return Err(Error::Archive(
            "control-journal archive carries trailing bytes".into(),
        ));
    }
    let actual_digest = digest.finalize();
    if footer
        != [
            state.entry_count,
            state.payload_bytes,
            state.first_sequence.unwrap_or_default(),
            state.last_sequence.unwrap_or_default(),
        ]
        || declared_digest != actual_digest
    {
        return Err(Error::Archive(
            "control-journal archive footer or digest do not match".into(),
        ));
    }
    Ok(state.inventory(hex(actual_digest), source_control_sequence, &families))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::control::ControlTransition;

    fn entry(sequence: u64, key: &str, replacement: Option<&[u8]>) -> ControlJournalEntry {
        ControlJournalEntry::committed(
            sequence,
            &ControlTransition {
                key: key.into(),
                expected: None,
                replacement: replacement.map(<[u8]>::to_vec),
                at: 100 + sequence,
                actor: "test".into(),
                action: "test.transition".into(),
                request_id: format!("request-{sequence}"),
                operation_id: format!("operation-{sequence}"),
            },
            None,
        )
    }

    fn families() -> Vec<String> {
        vec![
            "server/state/install".into(),
            "server/state/one/audit".into(),
            "server/state/one/audit-head".into(),
        ]
    }

    #[test]
    fn families_are_canonical_disjoint_and_outside_the_catalogue_scope() {
        validate_control_families(&families()).unwrap();
        validate_control_families(&[]).unwrap();
        assert!(validate_control_families(&["server/state/install/".into()]).is_err());
        assert!(validate_control_families(&["other/install".into()]).is_err());
        assert!(validate_control_families(&[
            "server/state/one/audit-head".into(),
            "server/state/one/audit".into(),
        ])
        .is_err());
        assert!(validate_control_families(&[
            "server/state/one".into(),
            "server/state/one/audit".into(),
        ])
        .is_err());
        assert!(validate_control_families(&["server/state/index-catalogue/scope".into()]).is_err());
        assert!(validate_control_families(&["server/state".into()]).is_err());
        assert!(control_family_covers(
            "server/state/estate",
            "server/state/estate/e/document"
        ));
        assert!(!control_family_covers(
            "server/state/estate",
            "server/state/estate-recovery/e"
        ));
    }

    #[test]
    fn archive_round_trips_entries_in_order_with_raw_values() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("control.rrd-control-archive");
        let entries = [
            entry(
                2,
                "server/state/install/record",
                Some(b"{\"installed\":true}"),
            ),
            entry(
                5,
                "server/state/one/audit/a",
                Some(&[0, 1, 2, 255, b'/', b'\n']),
            ),
            entry(6, "server/state/one/audit-head", Some(b"head")),
            entry(9, "server/state/one/audit/a", None),
        ];
        let mut writer = ControlArchiveWriter::create(&path, 12, &families()).unwrap();
        for entry in &entries {
            writer.entry(entry).unwrap();
        }
        let inventory = writer.finish().unwrap();
        assert_eq!(inventory.entry_count, 4);
        assert_eq!(inventory.payload_bytes, 18 + 6 + 4);
        assert_eq!(inventory.first_sequence, Some(2));
        assert_eq!(inventory.last_sequence, Some(9));
        assert_eq!(inventory.source_control_sequence, 12);

        let mut read = Vec::new();
        let inspected = read_control_archive(&path, |entry| {
            read.push(entry.clone());
            Ok(())
        })
        .unwrap();
        assert_eq!(inspected, inventory);
        assert_eq!(read, entries);
        assert_eq!(inspect_control_archive(&path).unwrap(), inventory);
    }

    #[test]
    fn writer_rejects_out_of_scope_descending_or_overlong_entries() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("control.rrd-control-archive");
        let mut writer = ControlArchiveWriter::create(&path, 10, &families()).unwrap();
        writer
            .entry(&entry(3, "server/state/install/record", Some(b"x")))
            .unwrap();
        assert!(writer
            .entry(&entry(3, "server/state/install/checkpoint", Some(b"x")))
            .is_err());
        assert!(writer
            .entry(&entry(4, "server/state/one/session/s", Some(b"x")))
            .is_err());
        assert!(writer
            .entry(&entry(11, "server/state/install/checkpoint", Some(b"x")))
            .is_err());
        let mut tampered = entry(4, "server/state/install/checkpoint", Some(b"x"));
        tampered.action = "tampered".into();
        assert!(writer.entry(&tampered).is_err());
        let inventory = writer.finish().unwrap();
        assert_eq!(inventory.entry_count, 1);
        assert_eq!(inspect_control_archive(&path).unwrap(), inventory);
    }

    #[test]
    fn empty_scope_archives_nothing_and_authenticates() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("empty.rrd-control-archive");
        let inventory = ControlArchiveWriter::create(&path, 0, &[])
            .unwrap()
            .finish()
            .unwrap();
        assert_eq!(inventory.entry_count, 0);
        assert_eq!(inventory.first_sequence, None);
        assert_eq!(inspect_control_archive(&path).unwrap(), inventory);
    }

    #[test]
    fn corruption_fails_closed() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("control.rrd-control-archive");
        let mut writer = ControlArchiveWriter::create(&path, 10, &families()).unwrap();
        writer
            .entry(&entry(3, "server/state/install/record", Some(b"value")))
            .unwrap();
        writer.finish().unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        let position = bytes
            .windows(5)
            .position(|window| window == b"value")
            .unwrap();
        bytes[position] = b'V';
        std::fs::write(&path, &bytes).unwrap();
        assert!(inspect_control_archive(&path).is_err());
        bytes.push(0);
        std::fs::write(&path, &bytes).unwrap();
        assert!(inspect_control_archive(&path).is_err());
    }
}
