use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{ensure, Context, Result};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OssRequirement {
    None,
    Optional,
    Required,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnvironmentRequirements {
    pub oss: OssRequirement,
    pub native_toolchain: bool,
}

impl EnvironmentRequirements {
    pub const fn basic(oss: OssRequirement) -> Self {
        Self {
            oss,
            native_toolchain: false,
        }
    }

    pub const fn native(oss: OssRequirement) -> Self {
        Self {
            oss,
            native_toolchain: true,
        }
    }
}

pub struct PreparedEnvironment {
    pub root: PathBuf,
    pub conda: PathBuf,
    pub jobs: usize,
    pub sccache_managed_cxx: bool,
}

impl PreparedEnvironment {
    pub fn prepare(requirements: EnvironmentRequirements) -> Result<Self> {
        let root = required_directory("PIXI_PROJECT_ROOT")?;
        let conda = required_directory("CONDA_PREFIX")?;
        let jobs = configured_jobs()?;

        env::set_var("BSC_BUILD_JOBS", jobs.to_string());
        env::set_var("GHCUP_INSTALL_BASE_PREFIX", root.join(".pixi"));
        env::set_var("CABAL_DIR", root.join(".pixi/cabal"));
        env::set_var("CABAL_CONFIG", root.join(".pixi/cabal/config"));
        env::set_var("MSYSTEM", "MINGW64");
        env::set_var("CHERE_INVOKING", "1");
        env::set_var("CARGO_TARGET_DIR", root.join(".pixi/tmp/cargo-target"));

        let conda_bin = conda.join("Library/bin");
        if requirements.native_toolchain {
            let ghcup_bin = ghcup_bindir(&root)?;
            env::set_var("BSC_GHCUP_BIN", &ghcup_bin);
            prepend_path([
                ghcup_bin,
                conda.join("Library/mingw-w64/bin"),
                conda.join("Library/usr/bin"),
                conda_bin.clone(),
            ])?;
        }

        if requirements.oss != OssRequirement::None {
            let config = root.join(".pixi/oss-cad-suite-root.txt");
            let oss_root = resolve_oss_root(&root, &config)?;
            if requirements.oss == OssRequirement::Required {
                let oss_root = oss_root.with_context(|| {
                    "OSS CAD Suite with iverilog/vvp was not configured; run \
                     'pixi run just configure-oss-cad-suite <path>' or set OSS_CAD_SUITE_ROOT"
                })?;
                configure_oss(&root, &conda_bin, &oss_root)?;
            } else if let Some(oss_root) = oss_root {
                configure_oss(&root, &conda_bin, &oss_root)?;
            }
        }

        let sccache = if cfg!(windows) {
            conda.join("bin/sccache.exe")
        } else {
            which::which("sccache").context("Pixi-managed sccache was not found on PATH")?
        };
        ensure!(
            sccache.is_file(),
            "Pixi-managed sccache was not found at {}",
            sccache.display()
        );
        ensure_msys_sccache_bridge(&conda, &sccache)?;
        set_default("SCCACHE_DIR", root.join(".pixi/cache/sccache"));
        set_default("SCCACHE_CACHE_SIZE", "10G");
        set_default("RUSTC_WRAPPER", &sccache);
        let sccache_managed_cxx = env::var_os("CXX").is_none();
        if sccache_managed_cxx {
            env::set_var("CXX", "sccache c++");
        }

        env::set_var(
            "PKG_CONFIG_PATH",
            env::join_paths([
                conda.join("Library/mingw-w64/lib/pkgconfig"),
                conda.join("Library/mingw-w64/share/pkgconfig"),
            ])?,
        );
        let ca_bundle = conda.join("Library/ssl/cacert.pem");
        env::set_var("SSL_CERT_FILE", &ca_bundle);
        env::set_var("GIT_SSL_CAINFO", &ca_bundle);
        env::set_var("CURL_CA_BUNDLE", &ca_bundle);

        Ok(Self {
            root,
            conda,
            jobs,
            sccache_managed_cxx,
        })
    }
}

fn ensure_msys_sccache_bridge(conda: &Path, sccache: &Path) -> Result<()> {
    if !cfg!(windows) {
        return Ok(());
    }

    // BSC's linker invokes MSYS make, whose /usr/bin search path does not
    // understand Pixi's Windows PATH. Keep the native executable in its package
    // directory for DLL lookup and bridge only its command name into MSYS.
    let bridge = conda.join("Library/usr/bin/sccache");
    let target = msys_path(sccache)?;
    let target = target.replace('\'', "'\\''");
    let contents = format!("#!/bin/sh\nexec '{target}' \"$@\"\n");
    if matches!(fs::read(&bridge), Ok(existing) if existing == contents.as_bytes()) {
        return Ok(());
    }

    match fs::remove_file(&bridge) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(error).with_context(|| {
                format!("could not refresh MSYS sccache bridge {}", bridge.display())
            });
        }
    }

    let temporary = bridge.with_file_name(format!(".sccache-bridge-{}.tmp", std::process::id()));
    fs::write(&temporary, &contents).with_context(|| {
        format!(
            "could not write temporary MSYS sccache bridge {}",
            temporary.display()
        )
    })?;
    match fs::rename(&temporary, &bridge) {
        Ok(()) => Ok(()),
        Err(_) if matches!(fs::read(&bridge), Ok(existing) if existing == contents.as_bytes()) => {
            let _ = fs::remove_file(&temporary);
            Ok(())
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(error).with_context(|| {
                format!("could not publish MSYS sccache bridge {}", bridge.display())
            })
        }
    }
}

fn msys_path(path: &Path) -> Result<String> {
    let path = path
        .to_str()
        .with_context(|| format!("path is not valid Unicode: {}", path.display()))?
        .replace('\\', "/");
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        let drive = (bytes[0] as char).to_ascii_lowercase();
        Ok(format!("/{drive}{}", &path[2..]))
    } else {
        Ok(path)
    }
}

fn ghcup_bindir(root: &Path) -> Result<PathBuf> {
    let ghcup =
        which::which("ghcup.exe").context("Pixi-managed ghcup.exe was not found on PATH")?;
    let output = Command::new(&ghcup)
        .args(["whereis", "bindir"])
        .current_dir(root)
        .output()
        .with_context(|| format!("could not query GHCup bindir using {}", ghcup.display()))?;
    ensure!(
        output.status.success(),
        "GHCup could not resolve its bindir: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    let output = String::from_utf8(output.stdout).context("GHCup bindir is not valid UTF-8")?;
    let bindir = normalize_windows_path(OsString::from(output.trim()));
    ensure!(
        bindir.is_dir(),
        "GHCup bindir does not exist: {}",
        bindir.display()
    );
    dunce::canonicalize(&bindir)
        .with_context(|| format!("could not resolve GHCup bindir: {}", bindir.display()))
}

fn required_directory(name: &str) -> Result<PathBuf> {
    let value = env::var_os(name).with_context(|| {
        format!("{name} is not set; run this task inside Pixi, for example 'pixi run just test'")
    })?;
    let path = PathBuf::from(value);
    ensure!(
        path.is_dir(),
        "{name} does not name a directory: {}",
        path.display()
    );
    dunce::canonicalize(&path)
        .with_context(|| format!("could not resolve {name}: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::msys_path;
    use std::path::Path;

    #[test]
    fn converts_windows_paths_for_msys() {
        assert_eq!(
            msys_path(Path::new(r"D:\project space\.pixi\sccache.exe")).unwrap(),
            "/d/project space/.pixi/sccache.exe"
        );
    }

    #[test]
    fn preserves_paths_without_a_windows_drive() {
        assert_eq!(
            msys_path(Path::new("/usr/bin/sccache")).unwrap(),
            "/usr/bin/sccache"
        );
    }
}

fn configured_jobs() -> Result<usize> {
    let default = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(16);
    let Some(value) = env::var_os("BSC_JOBS") else {
        return Ok(default);
    };
    let value = value
        .into_string()
        .map_err(|_| anyhow::anyhow!("BSC_JOBS must be valid Unicode"))?;
    let jobs = value
        .parse::<usize>()
        .with_context(|| format!("BSC_JOBS must be a positive integer, got {value:?}"))?;
    ensure!(jobs > 0, "BSC_JOBS must be a positive integer");
    Ok(jobs)
}

fn resolve_oss_root(project_root: &Path, config: &Path) -> Result<Option<PathBuf>> {
    let mut candidates = Vec::new();
    candidates.extend(env::var_os("OSS_CAD_SUITE_ROOT"));
    candidates.extend(env::var_os("YOSYSHQ_ROOT"));
    if config.is_file() {
        candidates.push(OsString::from(
            fs::read_to_string(config)
                .with_context(|| format!("could not read {}", config.display()))?
                .trim(),
        ));
    }
    if let Ok(iverilog) = which::which("iverilog.exe") {
        if let Some(root) = iverilog.parent().and_then(Path::parent) {
            candidates.push(root.as_os_str().to_owned());
        }
    }

    Ok(candidates
        .into_iter()
        .filter(|candidate| !candidate.is_empty())
        .map(normalize_windows_path)
        .map(|candidate| {
            if candidate.is_absolute() {
                candidate
            } else {
                project_root.join(candidate)
            }
        })
        .find(|candidate| valid_oss_root(candidate))
        .map(|path| dunce::canonicalize(&path).unwrap_or(path)))
}

pub fn save_oss_root(project_root: &Path, candidate: &Path) -> Result<PathBuf> {
    let candidate = normalize_windows_path(candidate.as_os_str().to_owned());
    let candidate = if candidate.is_absolute() {
        candidate
    } else {
        project_root.join(candidate)
    };
    ensure!(
        valid_oss_root(&candidate),
        "not a valid OSS CAD Suite root: {}",
        candidate.display()
    );
    let oss_root = dunce::canonicalize(&candidate).with_context(|| {
        format!(
            "could not resolve OSS CAD Suite root: {}",
            candidate.display()
        )
    })?;
    let config = project_root.join(".pixi/oss-cad-suite-root.txt");
    let parent = config
        .parent()
        .context("OSS CAD Suite configuration path has no parent")?;
    fs::create_dir_all(parent).with_context(|| format!("could not create {}", parent.display()))?;
    let text = oss_root
        .to_str()
        .context("OSS CAD Suite root is not valid UTF-8")?;
    fs::write(&config, format!("{text}\n"))
        .with_context(|| format!("could not write {}", config.display()))?;
    println!("Configured OSS CAD Suite: {}", oss_root.display());
    Ok(oss_root)
}

fn normalize_windows_path(candidate: OsString) -> PathBuf {
    let text = candidate.to_string_lossy();
    let text = text.trim().trim_matches('"');
    let bytes = text.as_bytes();
    if bytes.len() >= 2 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() {
        let drive = (bytes[1] as char).to_ascii_uppercase();
        let suffix = text.get(2..).unwrap_or_default().replace('/', "\\");
        PathBuf::from(format!("{drive}:{suffix}"))
    } else {
        PathBuf::from(text)
    }
}

fn valid_oss_root(root: &Path) -> bool {
    [
        root.join("environment.ps1"),
        root.join("bin/iverilog.exe"),
        root.join("bin/vvp.exe"),
        root.join("lib/ivl"),
    ]
    .iter()
    .all(|path| path.exists())
}

fn configure_oss(root: &Path, conda_bin: &Path, oss_root: &Path) -> Result<()> {
    let preferred_bin = root.join(".pixi/tools/pixi-preferred-bin");
    fs::create_dir_all(&preferred_bin)
        .with_context(|| format!("could not create {}", preferred_bin.display()))?;
    for name in [
        "z3.exe",
        "MSVCP140.dll",
        "VCRUNTIME140.dll",
        "VCRUNTIME140_1.dll",
    ] {
        let source = conda_bin.join(name);
        ensure!(
            source.is_file(),
            "Pixi-managed Z3 runtime file was not found: {}",
            source.display()
        );
        install_preferred_file(&source, &preferred_bin.join(name))?;
    }

    env::set_var("OSS_CAD_SUITE_ROOT", oss_root);
    env::set_var("BSC_OSS_CAD_SUITE_ROOT", oss_root);
    env::set_var("BSC_PIXI_PREFERRED_BIN", &preferred_bin);
    let mut yosys_root = oss_root.as_os_str().to_owned();
    yosys_root.push("\\");
    env::set_var("YOSYSHQ_ROOT", yosys_root);
    prepend_path([preferred_bin, oss_root.join("bin"), oss_root.join("lib")])?;
    Ok(())
}

fn install_preferred_file(source: &Path, destination: &Path) -> Result<()> {
    if destination.is_file() && files_equal(source, destination)? {
        return Ok(());
    }
    if destination.exists() {
        fs::remove_file(destination)
            .with_context(|| format!("could not replace {}", destination.display()))?;
    }
    if fs::hard_link(source, destination).is_err() {
        fs::copy(source, destination).with_context(|| {
            format!(
                "could not copy {} to {}",
                source.display(),
                destination.display()
            )
        })?;
    }
    Ok(())
}

fn files_equal(left: &Path, right: &Path) -> io::Result<bool> {
    if fs::metadata(left)?.len() != fs::metadata(right)?.len() {
        return Ok(false);
    }
    let mut left = BufReader::new(File::open(left)?);
    let mut right = BufReader::new(File::open(right)?);
    let mut left_buffer = [0_u8; 64 * 1024];
    let mut right_buffer = [0_u8; 64 * 1024];
    loop {
        let left_read = left.read(&mut left_buffer)?;
        let right_read = right.read(&mut right_buffer)?;
        if left_read != right_read || left_buffer[..left_read] != right_buffer[..right_read] {
            return Ok(false);
        }
        if left_read == 0 {
            return Ok(true);
        }
    }
}

fn prepend_path<const N: usize>(paths: [PathBuf; N]) -> Result<()> {
    let mut entries: Vec<_> = paths.into_iter().collect();
    if let Some(current) = env::var_os("PATH") {
        entries.extend(env::split_paths(&current));
    }
    env::set_var("PATH", env::join_paths(entries)?);
    Ok(())
}

fn set_default(name: &str, value: impl Into<OsString>) {
    if env::var_os(name).is_none() {
        env::set_var(name, value.into());
    }
}
