//! Card art kept on the player's PC (`card-cache/art/`), with a size cap:
//! when the folder grows past it, the least recently shown images go first.
//! Every file is checked again when read; a broken one is deleted.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::catalog::valid_id;

/// HearthstoneJSON's 256x JPGs are 5-30 KB; anything near this is not one.
pub const MAX_IMAGE_BYTES: usize = 512 * 1024;
/// The folder never stays above this (D-038).
pub const CACHE_CAP_BYTES: u64 = 50 * 1024 * 1024;

/// A JPEG starts with FF D8 FF.
pub fn is_jpeg(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes.len() <= MAX_IMAGE_BYTES && bytes.starts_with(&[0xFF, 0xD8, 0xFF])
}

/// FNV-1a, so two ids that differ only in case never share a file on
/// Windows' case-insensitive disk (that would be a wrong picture).
fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

pub struct ArtCache {
    dir: PathBuf,
    cap: u64,
}

impl ArtCache {
    pub fn new(dir: PathBuf, cap: u64) -> ArtCache {
        ArtCache { dir, cap }
    }

    fn path(&self, id: &str) -> Option<PathBuf> {
        valid_id(id).then(|| self.dir.join(format!("{id}-{:016x}.jpg", fnv1a(id))))
    }

    /// The cached image, or None. A file that is not a JPEG any more is
    /// removed so it is fetched again.
    pub fn read(&self, id: &str) -> Option<Vec<u8>> {
        let path = self.path(id)?;
        let bytes = fs::read(&path).ok()?;
        if !is_jpeg(&bytes) {
            let _ = fs::remove_file(&path);
            return None;
        }
        // Mark it as just used, for the cap.
        if let Ok(file) = fs::File::options().write(true).open(&path) {
            let _ = file.set_modified(SystemTime::now());
        }
        Some(bytes)
    }

    /// Writes to a side file first, so a crash never leaves half an image.
    pub fn write(&self, id: &str, bytes: &[u8]) -> io::Result<()> {
        let path = self
            .path(id)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "card id"))?;
        if !is_jpeg(bytes) {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "not a JPEG"));
        }
        fs::create_dir_all(&self.dir)?;
        let side = path.with_extension("part");
        fs::write(&side, bytes)?;
        fs::rename(&side, &path)?;
        self.prune();
        Ok(())
    }

    /// Above the cap, removes the least recently used images until the
    /// folder is at 80% of it.
    pub fn prune(&self) {
        let Ok(entries) = fs::read_dir(&self.dir) else {
            return;
        };
        let mut files: Vec<(SystemTime, u64, PathBuf)> = entries
            .filter_map(Result::ok)
            .filter(|e| is_cache_file(&e.path()))
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                let used = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                Some((used, meta.len(), e.path()))
            })
            .collect();
        let mut total: u64 = files.iter().map(|(_, len, _)| len).sum();
        if total <= self.cap {
            return;
        }
        let target = self.cap / 10 * 8;
        files.sort();
        for (_, len, path) in files {
            if total <= target {
                break;
            }
            if fs::remove_file(&path).is_ok() {
                total -= len;
            }
        }
    }

    /// Bytes the folder holds now (images only).
    pub fn size(&self) -> u64 {
        fs::read_dir(&self.dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .filter(|e| is_cache_file(&e.path()))
                    .filter_map(|e| e.metadata().ok())
                    .map(|m| m.len())
                    .sum()
            })
            .unwrap_or(0)
    }
}

fn is_cache_file(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "jpg" || e == "part")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    pub fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tl-cards-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    /// A made-up "JPEG": the magic bytes and filler, not a real image.
    pub fn fake_jpeg(len: usize) -> Vec<u8> {
        let mut bytes = vec![0xFF, 0xD8, 0xFF, 0xE0];
        bytes.resize(len.max(4), 0x11);
        bytes
    }

    #[test]
    fn an_image_written_reads_back() {
        let cache = ArtCache::new(temp_dir("roundtrip"), CACHE_CAP_BYTES);
        assert_eq!(cache.read("CARD_1"), None);
        cache.write("CARD_1", &fake_jpeg(100)).unwrap();
        assert_eq!(cache.read("CARD_1"), Some(fake_jpeg(100)));
    }

    #[test]
    fn only_jpegs_under_the_size_limit_are_kept() {
        let cache = ArtCache::new(temp_dir("checks"), CACHE_CAP_BYTES);
        assert!(cache.write("CARD_1", b"<html>").is_err());
        assert!(cache
            .write("CARD_1", &fake_jpeg(MAX_IMAGE_BYTES + 1))
            .is_err());
        assert!(
            cache.write("../CARD", &fake_jpeg(10)).is_err(),
            "never a path"
        );
        assert_eq!(cache.read("CARD_1"), None);
    }

    #[test]
    fn a_broken_file_on_disk_is_removed_and_not_shown() {
        let dir = temp_dir("broken");
        let cache = ArtCache::new(dir.clone(), CACHE_CAP_BYTES);
        cache.write("CARD_1", &fake_jpeg(10)).unwrap();
        let path = cache.path("CARD_1").unwrap();
        fs::write(&path, b"garbage").unwrap();
        assert_eq!(cache.read("CARD_1"), None);
        assert!(!path.exists());
    }

    #[test]
    fn ids_that_differ_only_in_case_never_share_a_file() {
        let cache = ArtCache::new(temp_dir("case"), CACHE_CAP_BYTES);
        let upper = cache.path("CARD_A").unwrap();
        let lower = cache.path("card_a").unwrap();
        let name = |p: &Path| p.file_name().unwrap().to_string_lossy().to_lowercase();
        assert_ne!(name(&upper), name(&lower));
    }

    #[test]
    fn past_the_cap_the_least_recently_used_images_go_first() {
        let dir = temp_dir("cap");
        // Room for three 100-byte images; at 80% of 350 two are kept.
        let cache = ArtCache::new(dir.clone(), 350);
        let old = SystemTime::now() - Duration::from_secs(3600);
        for (i, id) in ["OLDEST", "MIDDLE", "NEWEST"].iter().enumerate() {
            cache.write(id, &fake_jpeg(100)).unwrap();
            let file = fs::File::options()
                .write(true)
                .open(cache.path(id).unwrap())
                .unwrap();
            file.set_modified(old + Duration::from_secs(i as u64 * 60))
                .unwrap();
        }
        // Reading the oldest makes it the most recently used.
        assert!(cache.read("OLDEST").is_some());
        cache.write("FOURTH", &fake_jpeg(100)).unwrap();
        assert!(cache.size() <= 280);
        assert!(
            cache.read("MIDDLE").is_none(),
            "least recently used went first"
        );
        assert!(cache.read("NEWEST").is_none());
        assert!(cache.read("OLDEST").is_some());
        assert!(cache.read("FOURTH").is_some());
    }
}
