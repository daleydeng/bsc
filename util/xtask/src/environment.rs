use std::env;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Result};

pub struct PreparedEnvironment {
    pub root: PathBuf,
    pub jobs: usize,
    pub ccache_managed_cxx: bool,
}

impl PreparedEnvironment {
    pub fn prepare(requires_oss: bool) -> Result<Self> {
        let root = required_directory("PIXI_PROJECT_ROOT")?;
        let conda = required_directory("CONDA_PREFIX")?;
        let jobs = configured_jobs()?;

        env::set_var("BSC_BUILD_JOBS", jobs.to_string());
        env::set_var("CARGO_TARGET_DIR", root.join(".pixi/tmp/cargo-target"));

        let conda_bin = conda.join("Library/bin");
        if requires_oss {
            let config = root.join(".pixi/oss-cad-suite-root.txt");
            let oss_root = resolve_oss_root(&config)?.with_context(|| {
                "OSS CAD Suite with iverilog/vvp was not configured; run \
                 'pixi run just configure-oss-cad-suite <path>' or set OSS_CAD_SUITE_ROOT"
            })?;
            configure_oss(&root, &conda_bin, &oss_root)?;
        }

        which::which("ccache.exe").context("Pixi-managed ccache.exe was not found on PATH")?;
        set_default("CCACHE_DIR", root.join(".pixi/cache/ccache"));
        set_default("CCACHE_BASEDIR", &root);
        set_default("CCACHE_MAXSIZE", "10G");
        let ccache_managed_cxx = env::var_os("CXX").is_none();
        if ccache_managed_cxx {
            env::set_var("CXX", "ccache c++");
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
            jobs,
            ccache_managed_cxx,
        })
    }
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

fn resolve_oss_root(config: &Path) -> Result<Option<PathBuf>> {
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
        .map(normalize_windows_path)
        .find(|candidate| valid_oss_root(candidate))
        .map(|path| dunce::canonicalize(&path).unwrap_or(path)))
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
