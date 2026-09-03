use std::{
    env, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const CACHE_MAGIC: &[u8] = b"BURR_VIEWER_CACHE_V1\n";
const CACHE_DIRECTORY_VERSION: &str = "viewer-v1";
const CACHE_FILE_SUFFIX: &str = ".burr-viewer";
const MAX_CACHE_ENTRIES: usize = 128;
const MAX_CACHE_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_VIEWER_HTML_BYTES: usize = 64 * 1024 * 1024;
const MAX_CACHE_ENTRY_BYTES: usize = MAX_VIEWER_HTML_BYTES + 64 * 1024;

#[derive(Clone, Debug)]
pub struct ViewerCache {
    root: Option<PathBuf>,
}

impl ViewerCache {
    pub fn from_environment() -> Self {
        Self {
            root: cache_root_from_environment(),
        }
    }

    #[cfg(test)]
    fn at(root: PathBuf) -> Self {
        Self { root: Some(root) }
    }

    pub fn load(&self, key: &str) -> Result<Option<String>, String> {
        let Some(path) = self.entry_path(key) else {
            return Ok(None);
        };
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(format!(
                    "Failed to read viewer cache {}: {error}",
                    path.display()
                ));
            }
        };
        if bytes.len() > MAX_CACHE_ENTRY_BYTES || !bytes.starts_with(CACHE_MAGIC) {
            return Ok(None);
        }
        let length_start = CACHE_MAGIC.len();
        let length_end = length_start + 8;
        let Some(length_bytes) = bytes.get(length_start..length_end) else {
            return Ok(None);
        };
        let mut encoded_length = [0_u8; 8];
        encoded_length.copy_from_slice(length_bytes);
        let Ok(key_length) = usize::try_from(u64::from_le_bytes(encoded_length)) else {
            return Ok(None);
        };
        let key_end = length_end.saturating_add(key_length);
        let Some(stored_key) = bytes.get(length_end..key_end) else {
            return Ok(None);
        };
        if stored_key != key.as_bytes() {
            return Ok(None);
        }
        let Some(html) = bytes.get(key_end..) else {
            return Ok(None);
        };
        String::from_utf8(html.to_vec())
            .map(Some)
            .map_err(|error| format!("Viewer cache contained invalid UTF-8: {error}"))
    }

    pub fn store(&self, key: &str, html: &str) -> Result<bool, String> {
        let Some(path) = self.entry_path(key) else {
            return Ok(false);
        };
        if html.len() > MAX_VIEWER_HTML_BYTES {
            return Ok(false);
        }
        let total_bytes = CACHE_MAGIC.len() + 8 + key.len() + html.len();
        if total_bytes > MAX_CACHE_ENTRY_BYTES {
            return Ok(false);
        }
        let Some(root) = path.parent() else {
            return Ok(false);
        };
        fs::create_dir_all(root).map_err(|error| {
            format!("Failed to create viewer cache {}: {error}", root.display())
        })?;
        secure_directory(root)?;

        let mut bytes = Vec::with_capacity(total_bytes);
        bytes.extend_from_slice(CACHE_MAGIC);
        bytes.extend_from_slice(&(key.len() as u64).to_le_bytes());
        bytes.extend_from_slice(key.as_bytes());
        bytes.extend_from_slice(html.as_bytes());

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temporary = root.join(format!(".{}.{}.tmp", std::process::id(), nonce));
        fs::write(&temporary, bytes).map_err(|error| {
            format!(
                "Failed to write viewer cache {}: {error}",
                temporary.display()
            )
        })?;
        if let Err(error) = secure_file(&temporary) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        if let Err(error) = fs::rename(&temporary, &path) {
            let _ = fs::remove_file(&temporary);
            return Err(format!(
                "Failed to commit viewer cache {}: {error}",
                path.display()
            ));
        }
        prune_cache(root);
        Ok(true)
    }

    fn entry_path(&self, key: &str) -> Option<PathBuf> {
        self.root.as_ref().map(|root| {
            root.join(format!(
                "{}{CACHE_FILE_SUFFIX}",
                blake3::hash(key.as_bytes()).to_hex()
            ))
        })
    }
}

pub fn source_fingerprint(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|error| format!("Failed to read {} for caching: {error}", path.display()))?;
    let mut hash = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("Failed to read {} for caching: {error}", path.display()))?;
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    Ok(hash.finalize().to_hex().to_string())
}

fn cache_root_from_environment() -> Option<PathBuf> {
    if let Some(root) = env::var_os("BURR_CACHE_DIR") {
        return (!root.is_empty()).then(|| PathBuf::from(root).join(CACHE_DIRECTORY_VERSION));
    }

    #[cfg(target_os = "macos")]
    {
        env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join("Library/Caches/burr")
                .join(CACHE_DIRECTORY_VERSION)
        })
    }

    #[cfg(target_os = "windows")]
    {
        env::var_os("LOCALAPPDATA").map(|root| {
            PathBuf::from(root)
                .join("burr")
                .join(CACHE_DIRECTORY_VERSION)
        })
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(root) = env::var_os("XDG_CACHE_HOME") {
            return Some(
                PathBuf::from(root)
                    .join("burr")
                    .join(CACHE_DIRECTORY_VERSION),
            );
        }
        env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join(".cache/burr")
                .join(CACHE_DIRECTORY_VERSION)
        })
    }
}

fn prune_cache(root: &Path) {
    prune_cache_to_limits(root, MAX_CACHE_ENTRIES, MAX_CACHE_TOTAL_BYTES);
}

fn prune_cache_to_limits(root: &Path, max_entries: usize, max_bytes: u64) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut entries = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.ends_with(CACHE_FILE_SUFFIX))
        })
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            let modified = metadata.modified().ok()?;
            Some((modified, entry.path(), metadata.len()))
        })
        .collect::<Vec<_>>();
    let mut entry_count = entries.len();
    let mut total_bytes = entries
        .iter()
        .map(|(_, _, bytes)| *bytes)
        .fold(0_u64, u64::saturating_add);
    if entry_count <= max_entries && total_bytes <= max_bytes {
        return;
    }
    entries.sort_by_key(|(modified, _, _)| *modified);
    for (_, path, bytes) in entries {
        if entry_count <= max_entries && total_bytes <= max_bytes {
            break;
        }
        if fs::remove_file(path).is_ok() {
            entry_count = entry_count.saturating_sub(1);
            total_bytes = total_bytes.saturating_sub(bytes);
        }
    }
}

#[cfg(unix)]
fn secure_directory(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| format!("Failed to secure viewer cache {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn secure_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(unix)]
fn secure_file(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("Failed to secure viewer cache {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn secure_file(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn cache_round_trip_requires_the_exact_key() {
        let temp = tempdir().unwrap();
        let cache = ViewerCache::at(temp.path().to_path_buf());

        assert!(cache.store("model-a", "<html>A</html>").unwrap());
        assert_eq!(
            cache.load("model-a").unwrap().as_deref(),
            Some("<html>A</html>")
        );
        assert_eq!(cache.load("model-b").unwrap(), None);
    }

    #[cfg(unix)]
    #[test]
    fn cache_geometry_is_private_to_the_current_user() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempdir().unwrap();
        let cache_root = temp.path().join("viewer-v1");
        let cache = ViewerCache::at(cache_root.clone());
        cache
            .store("private-model", "<html>geometry</html>")
            .unwrap();
        let entry = cache.entry_path("private-model").unwrap();

        assert_eq!(
            fs::metadata(cache_root).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(entry).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn source_fingerprint_changes_when_same_length_content_changes() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("part.step");
        fs::write(&path, "AAAA").unwrap();
        let before = source_fingerprint(&path).unwrap();
        fs::write(&path, "BBBB").unwrap();

        assert_ne!(source_fingerprint(&path).unwrap(), before);
    }

    #[test]
    fn corrupted_cache_entry_is_ignored() {
        let temp = tempdir().unwrap();
        let cache = ViewerCache::at(temp.path().to_path_buf());
        let path = cache.entry_path("model-a").unwrap();
        fs::create_dir_all(temp.path()).unwrap();
        fs::write(path, "not a Burr viewer").unwrap();

        assert_eq!(cache.load("model-a").unwrap(), None);
    }

    #[test]
    fn pruning_enforces_entry_and_byte_limits() {
        let temp = tempdir().unwrap();
        for (name, contents) in [("a", "1234"), ("b", "5678"), ("c", "9012")] {
            fs::write(
                temp.path().join(format!("{name}{CACHE_FILE_SUFFIX}")),
                contents,
            )
            .unwrap();
        }

        prune_cache_to_limits(temp.path(), 2, 8);

        let remaining = fs::read_dir(temp.path()).unwrap().count();
        let bytes = fs::read_dir(temp.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter_map(|entry| entry.metadata().ok())
            .map(|metadata| metadata.len())
            .sum::<u64>();
        assert!(remaining <= 2);
        assert!(bytes <= 8);
    }
}
