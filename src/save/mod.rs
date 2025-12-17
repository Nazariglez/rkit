mod driver;
mod file_sys;
mod local_storage;

use brotli::{CompressorWriter, Decompressor};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[cfg(target_arch = "wasm32")]
use web_time::{SystemTime, UNIX_EPOCH};

#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

use utils::helpers::user_data_path;

use driver::*;

/// header len is [checksum:u32][timestamp:u64][sys_flags:u16][user_flags:u16]
const HEADER_LEN: usize = 4 + 8 + 2 + 2;

const TEMP_EXT: &str = "tmp";
const BACKUP_EXT: &str = "bak";
const SAVE_EXT: &str = "sav";

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SaveFlags: u16 {
        /// Serialize using bincode
        const BINARY     = 0b0000_0000_0000_0001;
        /// Compress using brotli
        const COMPRESSED = 0b0000_0000_0000_0010;
        /// Don't fail on checksum validation
        const PROTECTED  = 0b0000_0000_0000_0100;
    }
}

impl Default for SaveFlags {
    fn default() -> Self {
        SaveFlags::BINARY | SaveFlags::COMPRESSED | SaveFlags::PROTECTED
    }
}

#[derive(Debug, Clone)]
pub struct SaveMetadata {
    pub path: PathBuf,
    pub checksum: u32,
    pub timestamp: u64,
    pub sys_flags: SaveFlags,
    pub user_flags: Option<u16>,
}

#[derive(Debug)]
pub struct SaveData<D> {
    pub metadata: SaveMetadata,
    pub data: D,
}

#[inline]
fn data_dir(base_dir: &str) -> Result<PathBuf, String> {
    match std::env::var("SAVE_FILE_DIR").ok() {
        Some(p) => Ok(PathBuf::from(p).join(base_dir)),
        None => user_data_path(base_dir)
            .ok_or_else(|| "Unable to set a folder to save the savefile.".to_string()),
    }
}

#[inline]
fn hash_data(timestamp: u64, flags: SaveFlags, user_flags: u16, data: &[u8]) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&timestamp.to_le_bytes());
    hasher.update(&flags.bits().to_le_bytes());
    hasher.update(&user_flags.to_le_bytes());
    hasher.update(data);
    hasher.finalize()
}

#[inline]
fn is_valid_checksum(meta: &SaveMetadata, body: &[u8]) -> bool {
    let body_checksum = hash_data(
        meta.timestamp,
        meta.sys_flags,
        meta.user_flags.unwrap_or_default(),
        body,
    );
    body_checksum == meta.checksum
}

pub fn save_data_to_file<D>(
    base_dir: &str,
    slot: &str,
    data: &D,
    sys_flags: SaveFlags,
    user_flags: Option<u16>,
) -> Result<String, String>
where
    for<'a> D: Serialize + Deserialize<'a>,
{
    // default to 0 if there is no user flags passed in
    let user_flags = user_flags.unwrap_or_default();

    // get the OS data directory
    let dir = data_dir(base_dir)?;

    // create the game directory if needed
    SaveDriver::ensure_dir(&dir).map_err(|e| format!("Cannot create save directory: {e}"))?;

    // build filename with timestamp + flags
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();

    // serialize game data
    let raw = if sys_flags.contains(SaveFlags::BINARY) {
        bincode::serde::encode_to_vec(data, bincode::config::standard())
            .map_err(|e| format!("Data serialization as binary failed: {e}"))
    } else {
        serde_json::to_vec(data).map_err(|e| format!("Data serialization as json failed: {e}"))
    }?;

    // compress if needed
    let raw = if sys_flags.contains(SaveFlags::COMPRESSED) {
        // brotli support 1-11 levels of quality, but 5 seems to be the sweet sport for performance and size (1-11)
        const COMPRESSION_QUALITY: u32 = 5;
        let mut compressor = CompressorWriter::new(Vec::new(), 4096, COMPRESSION_QUALITY, 22);
        compressor
            .write_all(&raw)
            .map_err(|e| format!("Save compression failed: {e}"))?;
        compressor.into_inner()
    } else {
        raw
    };

    // compute the checksum for the header and the compressed data
    let checksum = hash_data(ts, sys_flags, user_flags, &raw);

    // write to a temp file, if the process is stopped by the os (power loss, etc...)
    // we do not corrupt the lastest save file
    let temp_filename = format!("{slot}.{ts}.{TEMP_EXT}");
    let tmp_filepath = dir.join(&temp_filename);
    {
        let mut buf = Vec::with_capacity(HEADER_LEN + raw.len());

        // we append to the compressed data the checksum as a u32 in little endian so we can read
        // later the checksum and check if the file was altered, detecting corruptions or manual
        // changes in the data
        // the final format is [checksum][timestamp][sys_flags][user_flags][data]
        buf.extend_from_slice(&checksum.to_le_bytes());
        buf.extend_from_slice(&ts.to_le_bytes());
        buf.extend_from_slice(&sys_flags.bits().to_le_bytes());
        buf.extend_from_slice(&user_flags.to_le_bytes());
        buf.extend_from_slice(&raw);

        SaveDriver::write_bytes(&tmp_filepath, &buf).map_err(|e| format!("Write error: {e}"))?;
    }

    let final_filepath = dir.join(slot).with_extension(SAVE_EXT);

    // rename the "old" main file as a new backup file
    let final_exists = SaveDriver::exists(&final_filepath).map_err(|e| e.to_string())?;
    if final_exists {
        let backup_filepath = dir.join(&temp_filename).with_extension(BACKUP_EXT);
        SaveDriver::rename(&final_filepath, &backup_filepath)
            .map_err(|e| format!("Rename main save file to backup file: {e}"))?;
        log::debug!("New backup file created '{backup_filepath:?}'");
    }

    // rename the temp file to the final file
    SaveDriver::rename(&tmp_filepath, &final_filepath)
        .map_err(|e| format!("Rename file error: {e}"))?;

    log::debug!("Save file created '{final_filepath:?}'");

    Ok(final_filepath.to_string_lossy().into_owned())
}

fn read_metadata(file_path: &Path) -> Result<SaveMetadata, String> {
    let raw = SaveDriver::read_bytes(file_path).map_err(|e| e.to_string())?;

    if raw.len() < HEADER_LEN {
        return Err(format!(
            "File too small: {} bytes (need at least {HEADER_LEN})",
            raw.len(),
        ));
    }

    let checksum = raw[0..4]
        .try_into()
        .map_err(|_| "Corrupt header: checksum slice has wrong length".to_string())
        .map(u32::from_le_bytes)?;

    let timestamp = raw[4..12]
        .try_into()
        .map_err(|_| "Corrupt header: timestamp slice has wrong length".to_string())
        .map(u64::from_le_bytes)?;

    let sys_flags_bits = raw[12..14]
        .try_into()
        .map_err(|_| "Corrupt header: sys flags slice has wrong length".to_string())
        .map(u16::from_le_bytes)?;
    let sys_flags = SaveFlags::from_bits(sys_flags_bits)
        .ok_or_else(|| format!("Unknown flags: {sys_flags_bits}"))?;

    let user_flags = raw[14..16]
        .try_into()
        .map_err(|_| "Corrupt header: user flags slice has wrong length".to_string())
        .map(u16::from_le_bytes)?;

    Ok(SaveMetadata {
        path: file_path.to_path_buf(),
        checksum,
        timestamp,
        sys_flags,
        user_flags: (user_flags != 0).then_some(user_flags),
    })
}

#[derive(Debug)]
pub struct SaveList {
    pub main: Option<SaveMetadata>,
    pub backups: Vec<SaveMetadata>,
}

impl SaveList {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn len(&self) -> usize {
        if self.main.is_some() {
            self.backups.len() + 1
        } else {
            self.backups.len()
        }
    }
}

fn list_saves(dir: &Path, slot: &str) -> Result<SaveList, String> {
    let entries = SaveDriver::read_dir(dir).map_err(|e| e.to_string())?;

    let mut main = None;
    let mut backups = entries
        .into_iter()
        .filter_map(|path| {
            let ext = path.extension()?.to_str()?;

            let is_main = ext == SAVE_EXT;
            let is_backup = ext == BACKUP_EXT;
            if !is_main && !is_backup {
                return None;
            }

            let stem = path.file_stem()?.to_str()?;
            let (name, _) = stem.rsplit_once('.').unwrap_or((stem, ""));
            if slot != name {
                return None;
            }

            match read_metadata(&path) {
                Ok(data) => {
                    if is_main {
                        main = Some(data);
                        None
                    } else {
                        Some(data)
                    }
                }
                Err(e) => {
                    log::warn!("Parsing save file '{path:?}': {e}");
                    None
                }
            }
        })
        .collect::<Vec<_>>();

    // sort by timestamp DESC
    backups.sort_by_key(|meta| meta.timestamp);
    backups.reverse();

    Ok(SaveList { main, backups })
}

fn parse_save_file<D>(meta: SaveMetadata) -> Result<SaveData<D>, String>
where
    D: for<'de> Deserialize<'de>,
{
    let raw = SaveDriver::read_bytes(&meta.path).map_err(|e| format!("File read error: {e}"))?;

    // return early if the file is too small even for the header
    if raw.len() < HEADER_LEN {
        return Err(format!(
            "File too small: {} bytes (need at least {HEADER_LEN})",
            raw.len(),
        ));
    }

    // split into header and body, so we can check the header to look for corruptions
    let (_header, body) = raw.split_at(HEADER_LEN);

    // check if the checksums are the same, otherwise means that the file could be
    // altered or corrupted
    let is_valid_checksum = is_valid_checksum(&meta, body);
    if !is_valid_checksum {
        log::warn!("Save file '{meta:?}' altered or corrupted.");

        if meta.sys_flags.contains(SaveFlags::PROTECTED) {
            return Err("Checksum mismatch, the file is corrupted".to_string());
        }
    }

    // decompress the data
    let data = if meta.sys_flags.contains(SaveFlags::COMPRESSED) {
        let mut data = vec![];
        Decompressor::new(body, 4096)
            .read_to_end(&mut data)
            .map_err(|e| format!("Data decompression fail: {e}"))?;
        data
    } else {
        body.to_vec()
    };

    // deserialize the data
    let data = if meta.sys_flags.contains(SaveFlags::BINARY) {
        bincode::serde::decode_from_slice(&data, bincode::config::standard())
            .map(|(data, _)| data)
            .map_err(|e| format!("Deserialization binary fail: {e}"))
    } else {
        serde_json::from_slice(&data).map_err(|e| format!("Deserialization json fail: {e}"))
    }?;

    Ok(SaveData {
        metadata: meta.clone(),
        data,
    })
}

pub fn load_last_saved_file<D>(base_dir: &str, slot: &str) -> Result<Option<SaveData<D>>, String>
where
    D: for<'de> Deserialize<'de>,
{
    let dir = data_dir(base_dir)?;

    // list all save files sorted by timestamp
    let list = list_saves(&dir, slot).map_err(|e| format!("Error reading save files: {e}"))?;
    if list.is_empty() {
        return Ok(None);
    }

    // helper function to parse the data and log errors
    let parse = |ctx: &str, save: SaveMetadata| match parse_save_file::<D>(save.clone()) {
        Ok(data) => Some(data),
        Err(err) => {
            log::warn!(
                "{ctx} save file seems corrupted, skipping '{:?}': {err}",
                save.path
            );
            None
        }
    };

    // first we check if there is a main save file available and non corrupted
    // if there is no main or is corrupted move to backups
    let parsed_meta = list.main.and_then(|save| parse("Main", save)).or_else(|| {
        list.backups
            .into_iter()
            .find_map(|save| parse("Backup", save))
    });

    let meta = parsed_meta.ok_or_else(|| "All saves files seems corrupted.".to_string())?;
    log::debug!("Loaded save file '{:?}'", meta.metadata.path);

    Ok(Some(meta))
}

pub fn clean_backups(base_dir: &str, slot: &str, keep: usize) -> Result<usize, String> {
    let dir = data_dir(base_dir)?;
    let mut backups = Vec::new();

    // find all backup files matching "slot.timestamp.bak"
    for path in SaveDriver::read_dir(&dir).map_err(|e| format!("Read save directory error: {e}"))? {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            if ext != BACKUP_EXT {
                continue;
            }
        } else {
            continue;
        }

        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            // Expecting "slot.timestamp"
            if let Some((name, ts_str)) = stem.rsplit_once('.')
                && name == slot
                && let Ok(ts) = ts_str.parse::<u64>()
            {
                backups.push((ts, path));
            }
        }
    }

    // Sort by timestamp descending
    backups.sort_by(|a, b| b.0.cmp(&a.0));

    // Delete any backups beyond the first `keep`
    let mut deleted = 0;
    for (_, path) in backups.iter().skip(keep) {
        SaveDriver::remove_file(path)
            .map_err(|e| format!("Failed to delete '{}': {e}", path.display()))?;
        deleted += 1;
    }

    Ok(deleted)
}

pub fn clear_save_files(base_dir: &str, slots: Option<&[&str]>) -> Result<usize, String> {
    let tmp_ext: &str = &format!(".{TEMP_EXT}");

    let dir = data_dir(base_dir)?;

    let mut deleted = 0;
    for path in SaveDriver::read_dir(&dir).map_err(|e| format!("Read save directory error: {e}"))? {
        let fname = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            // skip weird names
            None => continue,
        };

        // Always skip temp files
        if fname.ends_with(tmp_ext) {
            continue;
        }

        // if there are names passed in we filter by them
        if let Some(names) = slots {
            // save_name.timestamp
            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s,
                None => continue,
            };

            // remove timestamp if any
            let (save_name, _) = match stem.rsplit_once('.') {
                Some(pair) => pair,
                None => (stem, ""),
            };

            if !names.contains(&save_name) {
                continue;
            }
        }

        // at this point means we can delete the file because either it matches the name or there
        // were no names provided
        SaveDriver::remove_file(&path)
            .map_err(|e| format!("Failed to delete '{}': {e}", path.display()))?;
        deleted += 1;
    }

    Ok(deleted)
}

#[inline]
pub fn clear_all_save_files(base_dir: &str) -> Result<usize, String> {
    clear_save_files(base_dir, None)
}

#[inline]
pub fn exists_save_file(base_dir: &Path, slot: &str) -> Result<bool, String> {
    list_saves(base_dir, slot).map(|list| !list.is_empty())
}

pub fn pick_latest_save<D>(base_dir: &str, slots: &[&str]) -> Option<SaveData<D>>
where
    D: for<'de> Deserialize<'de>,
{
    slots
        .iter()
        .filter_map(|slot| {
            load_last_saved_file::<D>(base_dir, slot)
                .inspect_err(|e| log::warn!("Error loading save for slot '{slot}': {e}"))
                .ok()
                .flatten()
        })
        .max_by_key(|sd| sd.metadata.timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::{fs, fs::OpenOptions, path::PathBuf};
    use tempfile::TempDir;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct TestData {
        x: u32,
        y: String,
    }

    fn combined_sys_flags() -> [SaveFlags; 8] {
        [
            SaveFlags::empty(),
            SaveFlags::all(),
            SaveFlags::BINARY,
            SaveFlags::COMPRESSED,
            SaveFlags::PROTECTED,
            SaveFlags::BINARY | SaveFlags::COMPRESSED,
            SaveFlags::BINARY | SaveFlags::PROTECTED,
            SaveFlags::COMPRESSED | SaveFlags::PROTECTED,
        ]
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        combined_sys_flags().iter().for_each(|flags| {
            // set up an isolated temp dir
            let tmp = TempDir::new().unwrap();
            let base = tmp.path().to_str().unwrap();

            // no saves yet
            assert!(
                load_last_saved_file::<TestData>(base, "slot")
                    .unwrap()
                    .is_none()
            );

            // save
            let data = TestData {
                x: 42,
                y: "hello".into(),
            };
            let path = save_data_to_file(base, "slot", &data, *flags, None).unwrap();
            assert!(PathBuf::from(&path).exists());

            // load back
            let loaded = load_last_saved_file::<TestData>(base, "slot")
                .unwrap()
                .unwrap();
            assert_eq!(loaded.data, data);
            assert_eq!(loaded.metadata.sys_flags, *flags);
        });
    }

    #[test]
    fn test_clean_save_files() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().to_str().unwrap();

        let data = TestData {
            x: 24,
            y: "test".into(),
        };

        // create two different slots
        save_data_to_file(base, "A", &data, SaveFlags::empty(), None).unwrap();
        save_data_to_file(base, "B", &data, SaveFlags::empty(), None).unwrap();

        // clean just slot A
        let removed = clear_save_files(base, Some(&["A"])).unwrap();
        assert_eq!(removed, 1);

        // only B remains
        let list = list_saves(&PathBuf::from(base), "A").unwrap();
        assert_eq!(list.len(), 0);

        let list = list_saves(&PathBuf::from(base), "B").unwrap();
        assert_eq!(list.len(), 1);
        assert!(
            list.main
                .map(|data| {
                    data.path
                        .file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .starts_with("B")
                })
                .unwrap_or_default()
        );

        // now clean all
        let removed_all = clear_save_files(base, None).unwrap();
        assert_eq!(removed_all, 1);
        let empty_a = list_saves(&PathBuf::from(base), "A").unwrap();
        assert!(empty_a.is_empty());
        let empty_b = list_saves(&PathBuf::from(base), "B").unwrap();
        assert!(empty_b.is_empty());
    }

    #[test]
    fn test_corrupted_skipped_if_protected() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().to_str().unwrap();

        let data = TestData {
            x: 100,
            y: "oops".into(),
        };
        let path = save_data_to_file(base, "slot", &data, SaveFlags::PROTECTED, None).unwrap();

        // corrupt the first few bytes
        {
            let mut f = OpenOptions::new().write(true).open(&path).unwrap();
            f.write_all(&[0u8; 10]).unwrap();
            f.sync_all().unwrap();
        }

        // loading should skip the bad file and then fail (since it's the only one)
        let res = load_last_saved_file::<TestData>(base, "slot");
        assert!(res.is_err(), "Expected error when all saves are corrupted");
    }

    #[test]
    fn test_load_altered() {
        use std::{thread, time::Duration};

        let tmp = TempDir::new().unwrap();
        let base = tmp.path().to_str().unwrap();

        // Create the “older” save
        let data_old = TestData {
            x: 1,
            y: "old".into(),
        };
        let _old_path =
            save_data_to_file(base, "slot", &data_old, SaveFlags::empty(), None).unwrap();

        // Ensure a different timestamp for the next save
        thread::sleep(Duration::from_secs(1));

        // Create the newer save
        let data_new = TestData {
            x: 2,
            y: "new".into(),
        };
        let new_path =
            save_data_to_file(base, "slot", &data_new, SaveFlags::empty(), None).unwrap();

        // Alter the new file without corrupting it adding a few bytes
        {
            let mut f = OpenOptions::new().write(true).open(&new_path).unwrap();
            f.write_all(&[0u8; 10]).unwrap();
            f.sync_all().unwrap();
        }

        // should load the new file
        let loaded = load_last_saved_file::<TestData>(base, "slot")
            .unwrap()
            .unwrap();
        assert_eq!(loaded.data, data_new);
        assert_eq!(loaded.metadata.sys_flags, SaveFlags::empty());
    }

    #[test]
    fn test_skip_latest_and_load_older_if_protected() {
        use std::{thread, time::Duration};

        let tmp = TempDir::new().unwrap();
        let base = tmp.path().to_str().unwrap();

        // Create the “older” save
        let data_old = TestData {
            x: 1,
            y: "old".into(),
        };
        let _old_path =
            save_data_to_file(base, "slot", &data_old, SaveFlags::PROTECTED, None).unwrap();

        // Ensure a different timestamp for the next save
        thread::sleep(Duration::from_secs(1));

        // Create the newer save
        let data_new = TestData {
            x: 2,
            y: "new".into(),
        };
        let new_path =
            save_data_to_file(base, "slot", &data_new, SaveFlags::PROTECTED, None).unwrap();

        // Corrupt the newer file
        {
            let mut f = OpenOptions::new().write(true).open(&new_path).unwrap();
            // overwrite the header so it fails parsing
            f.write_all(&[0u8; 10]).unwrap();
            f.sync_all().unwrap();
        }

        // Try to load: should skip the corrupted “new” and return the “old”
        let loaded = load_last_saved_file::<TestData>(base, "slot")
            .unwrap()
            .unwrap();
        assert_eq!(loaded.data, data_old);
        assert_eq!(loaded.metadata.sys_flags, SaveFlags::PROTECTED);
    }

    #[test]
    fn test_clean_backups_keeps_last_n() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().to_str().unwrap();
        let slot = "testslot";

        const KEEP: usize = 5;
        const N: usize = 8;
        // Create save files with backups
        for _ in 0..N {
            // ensure unique timestamp
            std::thread::sleep(std::time::Duration::from_secs_f32(1.0));
            let _ = save_data_to_file(
                dir,
                slot,
                &TestData {
                    x: 999,
                    y: "32".to_string(),
                },
                SaveFlags::empty(),
                None,
            )
            .unwrap();
        }

        // chek that the number of save files is right
        let len = list_saves(&PathBuf::new().join(dir), slot).unwrap().len();
        assert_eq!(len, N);

        // now clean save files keeping KEEP files
        let removed = clean_backups(dir, slot, KEEP).unwrap();
        assert_eq!(removed, N - (KEEP + 1)); // +1 because main doesn't count

        // Check remaining backups
        let dir = data_dir(dir).unwrap();
        let entries: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|ent| ent.path()))
            .filter(|path| path.extension().and_then(|e| e.to_str()) == Some(BACKUP_EXT))
            .collect();
        assert_eq!(entries.len(), KEEP);

        // Ensure the 10 newest remain
        let mut timestamps: Vec<u64> = entries
            .iter()
            .filter_map(|path| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|stem| stem.rsplit_once('.').and_then(|(_, ts)| ts.parse().ok()))
            })
            .collect();
        timestamps.sort();

        // The smallest timestamp should be the 6th oldest of the original 15
        assert_eq!(timestamps.first().cloned().unwrap(), timestamps[0]);
    }

    #[test]
    fn test_backup_created_on_overwrite() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().to_str().unwrap();
        let slot = "my_slot";

        // First save: no backup should be created
        let _path1 = save_data_to_file(
            base,
            slot,
            &TestData {
                x: 1,
                y: "one".into(),
            },
            SaveFlags::empty(),
            None,
        )
        .unwrap();

        let dir = data_dir(base).unwrap();
        let backups: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|ent| ent.path()))
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(BACKUP_EXT))
            .collect();
        assert!(
            backups.is_empty(),
            "Expected no backups after first save, found: {:?}",
            backups
        );

        // Second save: should move the old .dat → .<ts>.bak
        std::thread::sleep(std::time::Duration::from_secs(1)); // ensure different ts
        let _path2 = save_data_to_file(
            base,
            slot,
            &TestData {
                x: 2,
                y: "two".into(),
            },
            SaveFlags::empty(),
            None,
        )
        .unwrap();

        let backups: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok().map(|ent| ent.path()))
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some(BACKUP_EXT))
            .collect();
        assert_eq!(
            backups.len(),
            1,
            "Expected exactly one backup after overwrite, found: {:?}",
            backups
        );

        // And that one backup’s file_stem starts with the slot name
        let stem = backups[0].file_stem().and_then(|s| s.to_str()).unwrap();
        assert!(
            stem.starts_with(slot),
            "Backup filename stem should start with `{slot}`, got `{stem}`",
        );
    }

    #[test]
    fn test_pick_latest_save_non_corrupted() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().to_str().unwrap();
        // First slot: "A"
        let data_a1 = TestData {
            x: 1,
            y: "one".into(),
        };
        save_data_to_file(base, "A", &data_a1, SaveFlags::empty(), None).unwrap();
        // Ensure a later timestamp
        std::thread::sleep(std::time::Duration::from_secs(1));
        let data_a2 = TestData {
            x: 2,
            y: "two".into(),
        };
        save_data_to_file(base, "A", &data_a2, SaveFlags::empty(), None).unwrap();

        // Second slot: "B"
        std::thread::sleep(std::time::Duration::from_secs(1));
        let data_b = TestData {
            x: 3,
            y: "three".into(),
        };
        save_data_to_file(base, "B", &data_b, SaveFlags::empty(), None).unwrap();

        // Now pick latest among [A, B]
        let result: Option<TestData> = pick_latest_save(base, &["A", "B"]).map(|r| r.data);

        // Should be data_b (timestamp newest)
        assert_eq!(result, Some(data_b));
    }

    #[test]
    fn test_pick_latest_save_skips_corrupted() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().to_str().unwrap();
        let good = TestData {
            x: 42,
            y: "good".into(),
        };
        let bad = TestData {
            x: 99,
            y: "bad".into(),
        };

        // Save good in slot "C"
        save_data_to_file(base, "C", &good, SaveFlags::empty(), None).unwrap();
        // Save bad and then corrupt it
        let path_bad = save_data_to_file(base, "D", &bad, SaveFlags::empty(), None).unwrap();
        // Corrupt file
        {
            use std::fs::OpenOptions;
            let mut f = OpenOptions::new().write(true).open(&path_bad).unwrap();
            f.write_all(&[0u8; 5]).unwrap();
            f.sync_all().unwrap();
        }

        // load_latest_save should return good only
        let result: Option<TestData> = pick_latest_save(base, &["C", "D"]).map(|r| r.data);
        assert_eq!(result, Some(good));
    }
}
