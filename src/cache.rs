use std::{
    env, fs,
    io::{self, BufWriter, Read, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

const CACHE_MAGIC: &[u8] = b"BURR_VIEWER_CACHE_V1\n";
const CACHE_DIRECTORY_VERSION: &str = "viewer-v1";
const CACHE_FILE_SUFFIX: &str = ".burr-viewer";
const MAX_CACHE_ENTRIES: usize = 128;
const MAX_MESH_CACHE_TOTAL_BYTES: u64 = 2 * 1024 * 1024 * 1024;
// Recent payloads may still be downloading in another viewer window.
const MESH_DOWNLOAD_GRACE: Duration = Duration::from_secs(10 * 60);
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
    pub(crate) fn at(root: PathBuf) -> Self {
        Self { root: Some(root) }
    }

    fn mesh_root(&self) -> PathBuf {
        self.root
            .clone()
            .unwrap_or_else(|| {
                std::env::temp_dir().join(format!("burr-session-{}", std::process::id()))
            })
            .join("meshes-v2")
    }

    pub fn mesh_path(&self, id: &str) -> Option<PathBuf> {
        (id.len() == 64
            && id
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()))
        .then(|| self.mesh_root().join(id))
    }

    /// Write one definition with bounded working memory; never expand occurrences.
    pub fn store_mesh(
        &self,
        geometry: &look::scene::Geometry,
    ) -> Result<serde_json::Value, String> {
        let root = self.mesh_root();
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        secure_directory(&root)?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temporary = root.join(format!(".{}.{}.tmp", std::process::id(), nonce));
        let result = (|| {
            let file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
                .map_err(|e| e.to_string())?;
            secure_file(&temporary)?;
            let mut writer = BufWriter::new(file);
            let mut hash = blake3::Hasher::new();
            let source_color = |index| {
                geometry
                    .source_attributes
                    .as_ref()
                    .and_then(|attributes| attributes.get(index))
                    .map(|a| a.color)
                    .unwrap_or([1.0; 4])
            };
            let first_color = source_color(0);
            let constant_color =
                (0..geometry.vertices.len()).all(|index| source_color(index) == first_color);
            let stride = if constant_color { 24 } else { 40 };
            // Little endian float32 position + normal, with RGBA only when it varies.
            for (index, vertex) in geometry.vertices.iter().enumerate() {
                let color = source_color(index);
                let mut record = [0_u8; 40];
                for (offset, value) in vertex
                    .position
                    .into_iter()
                    .chain(vertex.normal)
                    .chain(color)
                    .enumerate()
                {
                    record[offset * 4..offset * 4 + 4].copy_from_slice(&value.to_le_bytes());
                }
                hash.update(&record[..stride]);
                writer
                    .write_all(&record[..stride])
                    .map_err(|e| e.to_string())?;
            }
            for index in &geometry.indices {
                if *index as usize >= geometry.vertices.len() {
                    return Err("Mesh contains an out-of-range vertex index.".to_string());
                }
                let bytes = index.to_le_bytes();
                hash.update(&bytes);
                writer.write_all(&bytes).map_err(|e| e.to_string())?;
            }
            writer.flush().map_err(|e| e.to_string())?;
            let id = hash.finalize().to_hex().to_string();
            let destination = root.join(&id);
            // Replace even an existing entry: a truncated cached file must recover.
            fs::rename(&temporary, destination).map_err(|e| e.to_string())?;
            Ok(
                serde_json::json!({ "id": id, "vertices": geometry.vertices.len(), "indices": geometry.indices.len(), "stride": stride, "color": constant_color.then_some(first_color) }),
            )
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    pub fn meshes_available(&self, html: &str) -> bool {
        let Some((_, json)) = html.split_once("const burrManifest = ") else {
            return false;
        };
        let Some((json, _)) = json.split_once(";\n") else {
            return false;
        };
        let Ok(manifest) = serde_json::from_str::<serde_json::Value>(json) else {
            return false;
        };
        let Some(definitions) = manifest["definitions"].as_array() else {
            return false;
        };
        definitions.iter().all(|definition| {
            let Some(path) = definition["id"].as_str().and_then(|id| self.mesh_path(id)) else {
                return false;
            };
            let Some(vertices) = definition["vertices"].as_u64() else {
                return false;
            };
            let Some(indices) = definition["indices"].as_u64() else {
                return false;
            };
            let Some(stride @ (24 | 40)) = definition["stride"].as_u64() else {
                return false;
            };
            let expected = vertices
                .checked_mul(stride)
                .and_then(|v| indices.checked_mul(4).and_then(|i| v.checked_add(i)));
            fs::metadata(path)
                .ok()
                .is_some_and(|m| m.is_file() && Some(m.len()) == expected)
        })
    }

    pub fn load(&self, key: &str) -> Result<Option<String>, String> {
        let Some(path) = self.entry_path(key) else {
            return Ok(None);
        };
        let mut file = match fs::File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(format!(
                    "Failed to read viewer cache {}: {error}",
                    path.display()
                ))
            }
        };
        let size = file.metadata().map_err(|error| error.to_string())?.len();
        if size > MAX_CACHE_ENTRY_BYTES as u64 {
            return Ok(None);
        }
        let mut magic = vec![0_u8; CACHE_MAGIC.len()];
        if file.read_exact(&mut magic).is_err() || magic != CACHE_MAGIC {
            return Ok(None);
        }
        let mut encoded_length = [0_u8; 8];
        if file.read_exact(&mut encoded_length).is_err() {
            return Ok(None);
        }
        if u64::from_le_bytes(encoded_length) != key.len() as u64 {
            return Ok(None);
        }
        let mut stored_key = vec![0_u8; key.len()];
        if file.read_exact(&mut stored_key).is_err() || stored_key != key.as_bytes() {
            return Ok(None);
        }
        let mut html = String::new();
        file.take(MAX_VIEWER_HTML_BYTES as u64 + 1)
            .read_to_string(&mut html)
            .map_err(|error| format!("Viewer cache contained invalid HTML: {error}"))?;
        if html.len() > MAX_VIEWER_HTML_BYTES {
            return Ok(None);
        }
        Ok(Some(html))
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

        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temporary = root.join(format!(".{}.{}.tmp", std::process::id(), nonce));
        let written = (|| -> Result<(), String> {
            let file = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
                .map_err(|error| {
                    format!(
                        "Failed to create viewer cache {}: {error}",
                        temporary.display()
                    )
                })?;
            secure_file(&temporary)?;
            let mut writer = BufWriter::new(file);
            for bytes in [
                CACHE_MAGIC,
                &(key.len() as u64).to_le_bytes(),
                key.as_bytes(),
                html.as_bytes(),
            ] {
                writer.write_all(bytes).map_err(|error| error.to_string())?;
            }
            writer.flush().map_err(|error| error.to_string())
        })();
        if let Err(error) = written {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
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
        prune_meshes(
            &self.mesh_root(),
            MAX_MESH_CACHE_TOTAL_BYTES,
            MESH_DOWNLOAD_GRACE,
        );
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

fn prune_meshes(root: &Path, max_bytes: u64, grace: Duration) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut entries = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            if name.len() != 64 || !name.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            let metadata = entry.metadata().ok()?;
            if !metadata.is_file() {
                return None;
            }
            Some((metadata.modified().ok()?, entry.path(), metadata.len()))
        })
        .collect::<Vec<_>>();
    let mut total = entries
        .iter()
        .map(|(_, _, size)| *size)
        .fold(0_u64, u64::saturating_add);
    entries.sort_by_key(|(modified, _, _)| *modified);
    for (modified, path, size) in entries {
        if total <= max_bytes {
            break;
        }
        if modified.elapsed().unwrap_or_default() < grace {
            continue;
        }
        if fs::remove_file(path).is_ok() {
            total = total.saturating_sub(size);
        }
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
    fn mesh_pruning_protects_downloads_and_reclaims_expired_payloads() {
        let temp = tempdir().unwrap();
        for digit in ['a', 'b', 'c'] {
            fs::write(temp.path().join(digit.to_string().repeat(64)), "1234").unwrap();
        }
        prune_meshes(temp.path(), 8, MESH_DOWNLOAD_GRACE);
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 3);
        prune_meshes(temp.path(), 8, Duration::ZERO);
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 2);
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
