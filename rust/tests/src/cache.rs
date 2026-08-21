use crate::Toolchain;
use sha2::{Digest, Sha256};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, FileType};
use std::io::{self, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const FINGERPRINT_SCHEMA: &[u8] = b"bsc-rust-tests-scenario-cache-toolchain-v9";
const CASE_SCHEMA: &[u8] = b"bsc-rust-tests-scenario-result-v2";

const COPY_BUFFER_SIZE: usize = 64 * 1024;

pub(crate) struct ScenarioResultCache {
    enabled: bool,
    root: PathBuf,
    fingerprint: Option<[u8; 32]>,
    bluespecdir: PathBuf,
    command_path: OsString,
    hits: AtomicUsize,
    misses: AtomicUsize,
    stores: AtomicUsize,
}

#[derive(Debug)]
pub(crate) enum ReadOnlyCacheLookup {
    Disabled,
    Hit(PathBuf),
    Miss(CacheKey),
}

#[derive(Debug)]
enum CacheEntryLookup {
    Disabled,
    Hit { key: CacheKey, assertions: PathBuf },
    Miss(CacheKey),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct CacheKey([u8; 32]);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) struct CacheSummary {
    pub enabled: bool,
    pub hits: usize,
    pub misses: usize,
    pub stores: usize,
}

impl ScenarioResultCache {
    pub(crate) fn new(toolchain: &Toolchain) -> Result<Self, String> {
        let root = toolchain
            .project_root
            .join(".pixi")
            .join("cache")
            .join("rust-tests")
            .join("scenario-results")
            .join("v1");

        if !cache_enabled_from(env::var_os("BSC_TEST_CACHE").as_deref()) {
            return Ok(Self::disabled_at(root));
        }

        let fingerprint = fingerprint_toolchain(toolchain)?;
        let command_path = command_path(toolchain)?;
        fs::create_dir_all(&root)
            .map_err(|error| io_error("create scenario cache root", &root, error))?;

        Ok(Self {
            enabled: true,
            root,
            fingerprint: Some(fingerprint),
            bluespecdir: toolchain.bluespecdir.clone(),
            command_path,
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
            stores: AtomicUsize::new(0),
        })
    }

    pub(crate) fn lookup_read_only(
        &self,
        fixture_root: &Path,
        fixtures: &[&str],
        arguments: &[&str],
        log_path: &Path,
    ) -> Result<ReadOnlyCacheLookup, String> {
        match self.lookup_entry(fixture_root, fixtures, arguments)? {
            CacheEntryLookup::Disabled => Ok(ReadOnlyCacheLookup::Disabled),
            CacheEntryLookup::Hit { key, assertions } => {
                write_hit_log(log_path, &key)?;
                self.hits.fetch_add(1, Ordering::Relaxed);
                Ok(ReadOnlyCacheLookup::Hit(assertions))
            }
            CacheEntryLookup::Miss(key) => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                Ok(ReadOnlyCacheLookup::Miss(key))
            }
        }
    }

    fn lookup_entry(
        &self,
        fixture_root: &Path,
        fixtures: &[&str],
        arguments: &[&str],
    ) -> Result<CacheEntryLookup, String> {
        if !self.enabled {
            return Ok(CacheEntryLookup::Disabled);
        }

        let fingerprint = self
            .fingerprint
            .as_ref()
            .ok_or_else(|| "enabled scenario cache has no toolchain fingerprint".to_owned())?;
        let key = case_key(
            fingerprint,
            fixture_root,
            fixtures,
            arguments,
            &self.command_path,
            &self.bluespecdir,
        )?;
        let entry = self.root.join(key.hex());

        if !cache_entry_is_complete(&entry)? {
            remove_incomplete_entry(&entry)?;
            return Ok(CacheEntryLookup::Miss(key));
        }

        Ok(CacheEntryLookup::Hit {
            key,
            assertions: entry.join("assertions"),
        })
    }

    pub(crate) fn store(&self, key: &CacheKey, work_dir: &Path) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        fs::create_dir_all(&self.root)
            .map_err(|error| io_error("create scenario cache root", &self.root, error))?;
        let destination = self.root.join(key.hex());
        if path_exists(&destination)? {
            return Ok(());
        }

        let mut temporary = TemporaryDirectory::create(&self.root, &key.hex())?;
        let assertions = temporary.path().join("assertions");
        fs::create_dir(&assertions).map_err(|error| {
            io_error(
                "create scenario cache assertion directory",
                &assertions,
                error,
            )
        })?;
        let source = work_dir.join(".bsc-test-plan").join("assertions");
        if source.is_dir() {
            copy_directory_contents(&source, &assertions)?;
        }

        let marker = temporary.path().join(".complete");
        let mut marker_file = File::create(&marker)
            .map_err(|error| io_error("create cache completion marker", &marker, error))?;
        writeln!(marker_file, "scenario result cache entry complete")
            .and_then(|_| marker_file.sync_all())
            .map_err(|error| io_error("write cache completion marker", &marker, error))?;
        drop(marker_file);

        if path_exists(&destination)? {
            temporary.remove()?;
            return Ok(());
        }

        match fs::rename(temporary.path(), &destination) {
            Ok(()) => {
                temporary.disarm();
                self.stores.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(_error) if path_exists(&destination)? => {
                temporary.remove()?;
                Ok(())
            }
            Err(error) => Err(io_error(
                "publish scenario cache entry",
                &destination,
                error,
            )),
        }
    }

    pub(crate) fn summary(&self) -> CacheSummary {
        CacheSummary {
            enabled: self.enabled,
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            stores: self.stores.load(Ordering::Relaxed),
        }
    }

    fn disabled_at(root: PathBuf) -> Self {
        Self {
            enabled: false,
            root,
            fingerprint: None,
            bluespecdir: PathBuf::new(),
            command_path: OsString::new(),
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
            stores: AtomicUsize::new(0),
        }
    }
}

impl CacheKey {
    fn hex(&self) -> String {
        hex_digest(&self.0)
    }
}

fn cache_enabled_from(value: Option<&OsStr>) -> bool {
    value != Some(OsStr::new("0"))
}

fn command_path(toolchain: &Toolchain) -> Result<OsString, String> {
    let inherited = env::var_os("PATH").unwrap_or_default();
    let mut paths = Vec::new();
    if let Some(parent) = toolchain.bsc.parent() {
        paths.push(parent.to_path_buf());
    }
    paths.extend(env::split_paths(&inherited));
    env::join_paths(paths).map_err(|error| format!("construct scenario cache PATH: {error}"))
}

fn fingerprint_toolchain(toolchain: &Toolchain) -> Result<[u8; 32], String> {
    let mut hash = FramedHash::new();
    hash.field(b"schema", FINGERPRINT_SCHEMA);
    hash.field(b"os", env::consts::OS.as_bytes());
    hash.field(b"arch", env::consts::ARCH.as_bytes());
    hash_file(&mut hash, b"bsc executable", &toolchain.bsc)?;
    hash_file(&mut hash, b"bluetcl executable", &toolchain.bluetcl)?;
    hash_file(&mut hash, b"bsc2bsv executable", &toolchain.bsc2bsv)?;
    hash_file(&mut hash, b"vcdcheck executable", &toolchain.vcdcheck)?;
    match &toolchain.showrules {
        Some(showrules) => {
            hash.field(b"showrules marker", b"present");
            hash_file(&mut hash, b"showrules executable", showrules)?;
        }
        None => hash.field(b"showrules marker", b"missing"),
    }
    hash_file(&mut hash, b"make executable", &toolchain.make)?;
    hash_file(&mut hash, b"iverilog executable", &toolchain.iverilog)?;
    hash_file(&mut hash, b"C compiler executable", &toolchain.cc)?;

    hash.field(b"library tree", b"inst/lib");
    for entry in collect_tree(&toolchain.bluespecdir)? {
        hash.field(b"library entry path", &path_bytes(&entry.relative));
        if entry.file_type.is_dir() {
            hash.field(b"library entry type", b"directory");
        } else if entry.file_type.is_file() {
            hash.field(b"library entry type", b"file");
            hash_file(&mut hash, b"library file contents", &entry.full)?;
        } else {
            return Err(format!(
                "unsupported file type in BSC library tree: {}",
                entry.full.display()
            ));
        }
    }

    Ok(hash.finish())
}

fn case_key(
    fingerprint: &[u8; 32],
    fixture_root: &Path,
    fixtures: &[&str],
    arguments: &[&str],
    command_path: &OsStr,
    bluespecdir: &Path,
) -> Result<CacheKey, String> {
    let mut hash = FramedHash::new();
    hash.field(b"schema", CASE_SCHEMA);
    hash.field(b"toolchain fingerprint", fingerprint);
    hash.field(b"fixture count", &(fixtures.len() as u64).to_le_bytes());

    for fixture in fixtures {
        validate_fixture_path(fixture)?;
        let relative = Path::new(fixture);
        ensure_fixture_path_is_regular(fixture_root, relative)?;
        hash.field(b"fixture relative path", fixture.as_bytes());
        hash_file(&mut hash, b"fixture contents", &fixture_root.join(relative))?;
    }

    hash.field(b"argument count", &(arguments.len() as u64).to_le_bytes());
    for argument in arguments {
        hash.field(b"argument", argument.as_bytes());
    }

    hash.field(b"environment name", b"PATH");
    hash.field(b"environment value", &os_str_bytes(command_path));
    for name in [
        "BSC_OPTIONS",
        "BSC_PATH",
        "CC",
        "CFLAGS",
        "CXX",
        "CXXFLAGS",
        "BSC_CFLAGS",
        "BSC_CXXFLAGS",
        "MAKE",
        "BSC_MAKEFLAGS",
        "SYSTEMC",
        "LANG",
        "LC_ALL",
    ] {
        hash_optional_environment(&mut hash, name);
    }
    hash.field(b"environment name", b"BLUESPECDIR");
    hash.field(b"environment value", &os_str_bytes(bluespecdir.as_os_str()));
    hash.field(b"environment name", b"BSCTEST");
    hash.field(b"environment value", b"1");

    Ok(CacheKey(hash.finish()))
}

fn hash_optional_environment(hash: &mut FramedHash, name: &str) {
    hash.field(b"environment name", name.as_bytes());
    match env::var_os(name) {
        Some(value) => {
            hash.field(b"environment presence", b"set");
            hash.field(b"environment value", &os_str_bytes(&value));
        }
        None => hash.field(b"environment presence", b"unset"),
    }
}

fn validate_fixture_path(fixture: &str) -> Result<(), String> {
    let path = Path::new(fixture);
    if fixture.is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "fixture path must be a non-empty normalized relative path: {fixture:?}"
        ));
    }
    Ok(())
}

fn ensure_fixture_path_is_regular(root: &Path, relative: &Path) -> Result<(), String> {
    ensure_regular_directory(root, "fixture root")?;
    let mut current = root.to_path_buf();
    let component_count = relative.components().count();
    for (index, component) in relative.components().enumerate() {
        let Component::Normal(component) = component else {
            return Err(format!("invalid fixture path: {}", relative.display()));
        };
        current.push(component);
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| io_error("inspect fixture path", &current, error))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "fixture path contains a symlink: {}",
                current.display()
            ));
        }
        if index + 1 == component_count {
            if !metadata.is_file() {
                return Err(format!(
                    "fixture is not a regular file: {}",
                    current.display()
                ));
            }
        } else if !metadata.is_dir() {
            return Err(format!(
                "fixture parent is not a directory: {}",
                current.display()
            ));
        }
    }
    Ok(())
}

struct FramedHash(Sha256);

impl FramedHash {
    fn new() -> Self {
        Self(Sha256::new())
    }

    fn field(&mut self, name: &[u8], value: &[u8]) {
        self.part(name);
        self.part(value);
    }

    fn stream_field(&mut self, name: &[u8], path: &Path, length: u64) -> Result<(), String> {
        self.part(name);
        self.0.update(length.to_le_bytes());

        let mut file =
            File::open(path).map_err(|error| io_error("open file for hashing", path, error))?;
        let mut remaining = length;
        let mut buffer = [0_u8; COPY_BUFFER_SIZE];
        while remaining != 0 {
            let amount =
                usize::try_from(remaining.min(buffer.len() as u64)).unwrap_or(buffer.len());
            let read = file
                .read(&mut buffer[..amount])
                .map_err(|error| io_error("read file for hashing", path, error))?;
            if read == 0 {
                return Err(format!("file changed while hashing: {}", path.display()));
            }
            self.0.update(&buffer[..read]);
            remaining -= read as u64;
        }

        let mut extra = [0_u8; 1];
        if file
            .read(&mut extra)
            .map_err(|error| io_error("finish hashing file", path, error))?
            != 0
        {
            return Err(format!("file changed while hashing: {}", path.display()));
        }
        Ok(())
    }

    fn part(&mut self, bytes: &[u8]) {
        self.0.update((bytes.len() as u64).to_le_bytes());
        self.0.update(bytes);
    }

    fn finish(self) -> [u8; 32] {
        self.0.finalize().into()
    }
}

fn hash_file(hash: &mut FramedHash, field: &[u8], path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| io_error("inspect file for hashing", path, error))?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() || !file_type.is_file() {
        return Err(format!("cannot hash non-regular file: {}", path.display()));
    }
    hash.stream_field(field, path, metadata.len())
}

struct TreeEntry {
    relative: PathBuf,
    full: PathBuf,
    file_type: FileType,
}

fn collect_tree(root: &Path) -> Result<Vec<TreeEntry>, String> {
    ensure_regular_directory(root, "tree root")?;
    let mut entries = Vec::new();
    collect_tree_at(root, Path::new(""), &mut entries)?;
    Ok(entries)
}

fn collect_tree_at(
    root: &Path,
    relative_directory: &Path,
    entries: &mut Vec<TreeEntry>,
) -> Result<(), String> {
    let directory = root.join(relative_directory);
    let mut children = fs::read_dir(&directory)
        .map_err(|error| io_error("read directory", &directory, error))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| io_error("read directory entry", &directory, error))?;
    children.sort_by(|left, right| {
        os_str_bytes(&left.file_name()).cmp(&os_str_bytes(&right.file_name()))
    });

    for child in children {
        let relative = relative_directory.join(child.file_name());
        let full = root.join(&relative);
        let metadata = fs::symlink_metadata(&full)
            .map_err(|error| io_error("inspect tree entry", &full, error))?;
        let file_type = metadata.file_type();
        if file_type.is_symlink() {
            return Err(format!("symlink is not supported: {}", full.display()));
        }
        if !file_type.is_dir() && !file_type.is_file() {
            return Err(format!("special file is not supported: {}", full.display()));
        }

        entries.push(TreeEntry {
            relative: relative.clone(),
            full: full.clone(),
            file_type,
        });
        if file_type.is_dir() {
            collect_tree_at(root, &relative, entries)?;
        }
    }
    Ok(())
}

pub(crate) fn copy_directory_contents(source: &Path, destination: &Path) -> Result<(), String> {
    ensure_regular_directory(source, "copy source")?;
    ensure_regular_directory(destination, "copy destination")?;
    copy_directory_at(source, destination)
}

fn copy_directory_at(source: &Path, destination: &Path) -> Result<(), String> {
    let entries = fs::read_dir(source)
        .map_err(|error| io_error("read copy source directory", source, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| io_error("read copy source entry", source, error))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path)
            .map_err(|error| io_error("inspect copy source", &source_path, error))?;
        let file_type = metadata.file_type();

        if file_type.is_symlink() {
            return Err(format!("cannot copy symlink: {}", source_path.display()));
        }
        if file_type.is_dir() {
            ensure_or_create_destination_directory(&destination_path)?;
            copy_directory_at(&source_path, &destination_path)?;
        } else if file_type.is_file() {
            ensure_destination_can_be_file(&destination_path)?;
            fs::copy(&source_path, &destination_path).map_err(|error| {
                format!(
                    "copy {} to {}: {error}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        } else {
            return Err(format!(
                "cannot copy special file: {}",
                source_path.display()
            ));
        }
    }
    Ok(())
}

fn ensure_regular_directory(path: &Path, description: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| io_error(&format!("inspect {description}"), path, error))?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() || !file_type.is_dir() {
        return Err(format!(
            "{description} is not a regular directory: {}",
            path.display()
        ));
    }
    Ok(())
}

fn ensure_or_create_destination_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => Err(format!(
            "copy destination is not a regular directory: {}",
            path.display()
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => fs::create_dir(path)
            .map_err(|error| io_error("create copy destination directory", path, error)),
        Err(error) => Err(io_error("inspect copy destination", path, error)),
    }
}

fn ensure_destination_can_be_file(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => Err(format!(
            "copy destination is not a regular file: {}",
            path.display()
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("inspect copy destination", path, error)),
    }
}

fn cache_entry_is_complete(entry: &Path) -> Result<bool, String> {
    Ok(is_regular_file_if_present(&entry.join(".complete"))?
        && is_regular_directory_if_present(&entry.join("assertions"))?)
}

fn remove_incomplete_entry(entry: &Path) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(entry) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(io_error("inspect incomplete cache entry", entry, error)),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        fs::remove_file(entry).map_err(|error| io_error("remove invalid cache entry", entry, error))
    } else {
        fs::remove_dir_all(entry)
            .map_err(|error| io_error("remove incomplete cache entry", entry, error))
    }
}

fn is_regular_file_if_present(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(!metadata.file_type().is_symlink() && metadata.is_file()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error("inspect cache file", path, error)),
    }
}

fn is_regular_directory_if_present(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(!metadata.file_type().is_symlink() && metadata.is_dir()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error("inspect cache directory", path, error)),
    }
}

fn write_hit_log(path: &Path, key: &CacheKey) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| io_error("create cache-hit log directory", parent, error))?;
    }
    let mut log =
        File::create(path).map_err(|error| io_error("create cache-hit log", path, error))?;
    writeln!(log, "scenario result cache: hit")
        .and_then(|_| writeln!(log, "key: {}", key.hex()))
        .and_then(|_| writeln!(log, "duration: 0.000s"))
        .and_then(|_| log.flush())
        .map_err(|error| io_error("write cache-hit log", path, error))
}

fn path_exists(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error("inspect path", path, error)),
    }
}

struct TemporaryDirectory {
    path: PathBuf,
    armed: bool,
}

impl TemporaryDirectory {
    fn create(parent: &Path, key: &str) -> Result<Self, String> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);

        for _ in 0..100 {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = parent.join(format!(
                ".tmp-{key}-{}-{timestamp}-{sequence}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path, armed: true }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(io_error("create temporary cache directory", &path, error));
                }
            }
        }

        Err(format!(
            "could not create a unique temporary cache directory under {}",
            parent.display()
        ))
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn remove(&mut self) -> Result<(), String> {
        if self.armed {
            fs::remove_dir_all(&self.path)
                .map_err(|error| io_error("remove temporary cache directory", &self.path, error))?;
            self.armed = false;
        }
        Ok(())
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn hex_digest(digest: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(64);
    for byte in digest {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
}

#[cfg(windows)]
fn os_str_bytes(value: &OsStr) -> Vec<u8> {
    use std::os::windows::ffi::OsStrExt;

    value
        .encode_wide()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<_>>()
}

#[cfg(unix)]
fn os_str_bytes(value: &OsStr) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;

    value.as_bytes().to_vec()
}

#[cfg(not(any(unix, windows)))]
fn os_str_bytes(value: &OsStr) -> Vec<u8> {
    value.to_string_lossy().as_bytes().to_vec()
}

fn path_bytes(path: &Path) -> Vec<u8> {
    os_str_bytes(path.as_os_str())
}

fn io_error(action: &str, path: &Path, error: io::Error) -> String {
    format!("{action} {}: {error}", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assertion_snapshot_hits_are_thread_safe_and_exclude_work_products() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ScenarioResultCache>();

        let temp = TestDirectory::new("read-only-hit");
        let fixture_root = temp.path().join("fixtures");
        let first_work = temp.path().join("first-work");
        let untouched_work = temp.path().join("untouched-work");
        fs::create_dir_all(&fixture_root).unwrap();
        fs::create_dir_all(&first_work).unwrap();
        fs::create_dir_all(&untouched_work).unwrap();
        fs::write(fixture_root.join("Top.bsv"), "module mkTop; endmodule\n").unwrap();
        fs::write(first_work.join("generated.o"), vec![0_u8; 1024 * 1024]).unwrap();
        let asserted = first_work.join(".bsc-test-plan/assertions/0/1/generated.v");
        fs::create_dir_all(asserted.parent().unwrap()).unwrap();
        fs::write(&asserted, "module mkTop; endmodule\n").unwrap();

        let cache = test_cache(temp.path().join("cache"), true);
        let key = miss_key(
            &cache,
            &fixture_root,
            &["Top.bsv"],
            &["-u", "Top.bsv"],
            &first_work,
            temp.path(),
        );
        cache.store(&key, &first_work).unwrap();
        let cache_before = snapshot_tree(&cache.root);

        let assertion_roots = std::thread::scope(|scope| {
            let cache = &cache;
            let fixture_root = &fixture_root;
            let temp_root = temp.path();
            let handles = (0..8)
                .map(|index| {
                    scope.spawn(move || {
                        match cache
                            .lookup_read_only(
                                fixture_root,
                                &["Top.bsv"],
                                &["-u", "Top.bsv"],
                                &temp_root.join(format!("hit-{index}.log")),
                            )
                            .unwrap()
                        {
                            ReadOnlyCacheLookup::Hit(assertions) => assertions,
                            other => panic!("expected read-only cache hit, got {other:?}"),
                        }
                    })
                })
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|handle| handle.join().unwrap())
                .collect::<Vec<_>>()
        });

        assert!(assertion_roots
            .iter()
            .all(|path| path == &assertion_roots[0]));
        assert_eq!(
            fs::read_to_string(assertion_roots[0].join("0/1/generated.v")).unwrap(),
            "module mkTop; endmodule\n"
        );
        assert!(!assertion_roots[0].join("generated.o").exists());
        assert!(fs::read_dir(&untouched_work).unwrap().next().is_none());
        assert_eq!(snapshot_tree(&cache.root), cache_before);
        assert_eq!(
            cache.summary(),
            CacheSummary {
                enabled: true,
                hits: 8,
                misses: 1,
                stores: 1,
            }
        );
    }

    #[test]
    fn fixture_and_argument_changes_produce_misses() {
        let temp = TestDirectory::new("keys");
        let fixture_root = temp.path().join("fixtures");
        let work = temp.path().join("work");
        fs::create_dir_all(&fixture_root).unwrap();
        fs::create_dir_all(&work).unwrap();
        fs::write(fixture_root.join("Top.bsv"), "first\n").unwrap();
        fs::write(work.join("generated.v"), "generated\n").unwrap();

        let cache = test_cache(temp.path().join("cache"), true);
        let original = miss_key(
            &cache,
            &fixture_root,
            &["Top.bsv"],
            &["-u"],
            &work,
            temp.path(),
        );
        cache.store(&original, &work).unwrap();

        fs::write(fixture_root.join("Top.bsv"), "second\n").unwrap();
        let changed_fixture = miss_key(
            &cache,
            &fixture_root,
            &["Top.bsv"],
            &["-u"],
            &work,
            temp.path(),
        );
        assert_ne!(original, changed_fixture);

        fs::write(fixture_root.join("Top.bsv"), "first\n").unwrap();
        let changed_argument = miss_key(
            &cache,
            &fixture_root,
            &["Top.bsv"],
            &["-u", "-verilog"],
            &work,
            temp.path(),
        );
        assert_ne!(original, changed_argument);
    }

    #[test]
    fn zero_environment_value_disables_cache_without_hashing() {
        assert!(!cache_enabled_from(Some(OsStr::new("0"))));
        assert!(cache_enabled_from(None));
        assert!(cache_enabled_from(Some(OsStr::new("1"))));

        let temp = TestDirectory::new("disabled");
        let cache = test_cache(temp.path().join("cache"), false);

        assert!(matches!(
            cache
                .lookup_read_only(
                    &temp.path().join("missing-fixtures"),
                    &["missing.bsv"],
                    &["-u"],
                    &temp.path().join("missing-read-only.log"),
                )
                .unwrap(),
            ReadOnlyCacheLookup::Disabled
        ));
        assert_eq!(
            cache.summary(),
            CacheSummary {
                enabled: false,
                hits: 0,
                misses: 0,
                stores: 0,
            }
        );
        assert!(!temp.path().join("cache").exists());
    }

    fn miss_key(
        cache: &ScenarioResultCache,
        fixture_root: &Path,
        fixtures: &[&str],
        arguments: &[&str],
        _work_dir: &Path,
        temp: &Path,
    ) -> CacheKey {
        match cache
            .lookup_read_only(fixture_root, fixtures, arguments, &temp.join("miss.log"))
            .unwrap()
        {
            ReadOnlyCacheLookup::Miss(key) => key,
            other => panic!("expected cache miss, got {other:?}"),
        }
    }

    fn test_cache(root: PathBuf, enabled: bool) -> ScenarioResultCache {
        if enabled {
            fs::create_dir_all(&root).unwrap();
        }
        ScenarioResultCache {
            enabled,
            root,
            fingerprint: enabled.then_some([7; 32]),
            bluespecdir: PathBuf::from("test-bluespecdir"),
            command_path: OsString::from("test-path"),
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
            stores: AtomicUsize::new(0),
        }
    }

    fn snapshot_tree(root: &Path) -> Vec<(PathBuf, Option<Vec<u8>>)> {
        collect_tree(root)
            .unwrap()
            .into_iter()
            .map(|entry| {
                let contents = entry
                    .file_type
                    .is_file()
                    .then(|| fs::read(&entry.full).unwrap());
                (entry.relative, contents)
            })
            .collect()
    }

    #[test]
    fn toolchain_fingerprint_hashes_bluetcl_contents() {
        let temp = TestDirectory::new("bluetcl-fingerprint");
        let tools = temp.path().join("tools");
        let library = temp.path().join("lib");
        fs::create_dir_all(&tools).unwrap();
        fs::create_dir_all(&library).unwrap();
        fs::write(library.join("Prelude.bo"), "library").unwrap();
        for tool in [
            "bsc",
            "bluetcl",
            "bsc2bsv",
            "vcdcheck",
            "showrules",
            "make",
            "iverilog",
            "cc",
        ] {
            fs::write(tools.join(tool), tool).unwrap();
        }
        let mut toolchain = Toolchain {
            project_root: temp.path().to_owned(),
            bsc: tools.join("bsc"),
            bluetcl: tools.join("bluetcl"),
            bsc2bsv: tools.join("bsc2bsv"),
            dumpbo: PathBuf::new(),
            dumpba: PathBuf::new(),
            vcdcheck: tools.join("vcdcheck"),
            showrules: Some(tools.join("showrules")),
            make: tools.join("make"),
            iverilog: tools.join("iverilog"),
            bluespecdir: library,
            systemc_include: PathBuf::new(),
            systemc_lib: PathBuf::new(),
            cc: tools.join("cc"),
            cxx: PathBuf::new(),
        };

        let before = fingerprint_toolchain(&toolchain).unwrap();
        fs::write(&toolchain.bluetcl, "changed bluetcl").unwrap();
        let after_bluetcl = fingerprint_toolchain(&toolchain).unwrap();
        assert_ne!(before, after_bluetcl);

        let showrules = toolchain.showrules.clone().unwrap();
        fs::write(&showrules, "changed showrules").unwrap();
        let after_showrules = fingerprint_toolchain(&toolchain).unwrap();
        assert_ne!(after_bluetcl, after_showrules);

        toolchain.showrules = None;
        let missing_showrules = fingerprint_toolchain(&toolchain).unwrap();
        assert_ne!(after_showrules, missing_showrules);
    }

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = test_temp_root().join(format!(
                "bsc-scenario-cache-{label}-{}-{timestamp}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn test_temp_root() -> PathBuf {
        #[cfg(windows)]
        if let Some(local_app_data) = env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local_app_data).join("Temp");
        }

        env::temp_dir()
    }
}
