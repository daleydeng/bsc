#[cfg(windows)]
use process_wrap::std::JobObject;
#[cfg(unix)]
use process_wrap::std::ProcessGroup;
use process_wrap::std::{ChildWrapper, CommandWrap};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

mod assertion;
mod bluesim;
pub(crate) mod cache;
pub mod inventory;
pub mod test_plan;
mod vcd;

pub fn secure_directory_within(
    root: &Path,
    relative: &Path,
    label: &str,
) -> Result<PathBuf, String> {
    let root = fs::canonicalize(root)
        .map_err(|error| format!("canonicalize secure root {}: {error}", root.display()))?;
    let path = root.join(relative);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("inspect {label} {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || is_reparse_point(&metadata) || !metadata.is_dir() {
        return Err(format!(
            "{label} must be a non-symbolic-link directory: {}",
            path.display()
        ));
    }
    let canonical = fs::canonicalize(&path)
        .map_err(|error| format!("canonicalize {label} {}: {error}", path.display()))?;
    if !canonical.starts_with(&root) {
        return Err(format!("{label} escapes secure root: {}", path.display()));
    }
    Ok(canonical)
}

pub fn secure_file_within(root: &Path, relative: &Path, label: &str) -> Result<PathBuf, String> {
    let root = fs::canonicalize(root)
        .map_err(|error| format!("canonicalize secure root {}: {error}", root.display()))?;
    let path = root.join(relative);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("inspect {label} {}: {error}", path.display()))?;
    if metadata.file_type().is_symlink() || is_reparse_point(&metadata) || !metadata.is_file() {
        return Err(format!(
            "{label} must be a regular non-symbolic-link file: {}",
            path.display()
        ));
    }
    let canonical = fs::canonicalize(&path)
        .map_err(|error| format!("canonicalize {label} {}: {error}", path.display()))?;
    if !canonical.starts_with(&root) {
        return Err(format!("{label} escapes secure root: {}", path.display()));
    }
    Ok(canonical)
}

fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        let _ = metadata;
        false
    }
}

pub fn secure_read_file(root: &Path, relative: &Path, label: &str) -> Result<Vec<u8>, String> {
    let path = secure_file_within(root, relative, label)?;
    fs::read(&path).map_err(|error| format!("read {label} {}: {error}", path.display()))
}

#[derive(Debug, Clone)]
pub struct Toolchain {
    pub project_root: PathBuf,
    pub bsc: PathBuf,
    pub bluetcl: PathBuf,
    pub bsc2bsv: PathBuf,
    pub dumpbo: PathBuf,
    pub dumpba: PathBuf,
    pub vcdcheck: PathBuf,
    pub showrules: Option<PathBuf>,
    pub make: PathBuf,
    pub iverilog: PathBuf,
    pub bluespecdir: PathBuf,
    pub systemc_include: PathBuf,
    pub systemc_lib: PathBuf,
    pub cc: PathBuf,
    pub cxx: PathBuf,
}

impl Toolchain {
    pub fn discover() -> Result<Self, String> {
        let project_root = locate_project_root()?;
        let core_dir = project_root.join("inst").join("bin").join("core");
        let bsc = match env::var_os("BSC_UNDER_TEST") {
            Some(configured) => {
                let configured = PathBuf::from(configured);
                let candidate = if configured.is_absolute() {
                    configured
                } else {
                    project_root.join(configured)
                };
                if !candidate.is_file() {
                    return Err(format!(
                        "BSC_UNDER_TEST does not point to a file: {}",
                        candidate.display()
                    ));
                }
                candidate
            }
            None => [core_dir.join("bsc.exe"), core_dir.join("bsc")]
                .into_iter()
                .find(|candidate| candidate.is_file())
                .ok_or_else(|| {
                    format!(
                        "BSC is not built under {}; run `pixi run just build` first",
                        core_dir.display()
                    )
                })?,
        };
        let bluetcl =
            discover_companion_tool(&project_root, &bsc, "BLUETCL_UNDER_TEST", "bluetcl")?;
        let bsc2bsv =
            discover_companion_tool(&project_root, &bsc, "BSC2BSV_UNDER_TEST", "bsc2bsv")?;
        let dumpbo = discover_companion_tool(&project_root, &bsc, "DUMPBO_UNDER_TEST", "dumpbo")?;
        let dumpba = discover_companion_tool(&project_root, &bsc, "DUMPBA_UNDER_TEST", "dumpba")?;
        let vcdcheck =
            discover_companion_tool(&project_root, &bsc, "VCDCHECK_UNDER_TEST", "vcdcheck")?;
        let showrules_override = env::var_os("SHOWRULES_UNDER_TEST")
            .map(|path| ("SHOWRULES_UNDER_TEST", path))
            .or_else(|| env::var_os("SHOWRULES").map(|path| ("SHOWRULES", path)));
        let showrules =
            discover_optional_companion_tool(&project_root, &bsc, showrules_override, "showrules")?;
        let bluespecdir = project_root.join("inst").join("lib");
        if !bluespecdir.is_dir() {
            return Err(format!(
                "BSC library directory is missing at {}; run `pixi run just build` first",
                bluespecdir.display()
            ));
        }
        let pixi_prefix = discover_pixi_prefix(&project_root)?;
        let make = discover_make(&project_root, &pixi_prefix)?;
        let iverilog = discover_iverilog(&project_root, &pixi_prefix)?;
        let systemc_include = discover_pixi_directory(
            &pixi_prefix,
            &["Library/include", "include"],
            "SystemC include",
        )?;
        let systemc_header = systemc_include.join("systemc");
        if !systemc_header.is_file() {
            return Err(format!(
                "SystemC header is missing at {}; install the Pixi-provided SystemC package",
                systemc_header.display()
            ));
        }
        let systemc_lib =
            discover_pixi_directory(&pixi_prefix, &["Library/lib", "lib"], "SystemC library")?;
        if ![
            systemc_lib.join("libsystemc.dll.a"),
            systemc_lib.join("libsystemc.a"),
            systemc_lib.join("systemc.lib"),
        ]
        .iter()
        .any(|candidate| candidate.is_file())
        {
            return Err(format!(
                "SystemC library is missing under {}; install the Pixi-provided SystemC package",
                systemc_lib.display()
            ));
        }
        let cc = discover_pixi_file(
            &pixi_prefix,
            &[
                "Library/mingw-w64/bin/gcc.exe",
                "mingw64/bin/gcc.exe",
                "usr/bin/gcc.exe",
                "bin/gcc.exe",
                "bin/gcc",
            ],
            "C compiler",
        )?;
        let cxx = discover_pixi_file(
            &pixi_prefix,
            &[
                "Library/mingw-w64/bin/g++.exe",
                "mingw64/bin/g++.exe",
                "usr/bin/g++.exe",
                "bin/g++.exe",
                "bin/g++",
            ],
            "C++ compiler",
        )?;

        Ok(Self {
            project_root,
            bsc,
            bluetcl,
            bsc2bsv,
            dumpbo,
            dumpba,
            vcdcheck,
            showrules,
            make,
            iverilog,
            bluespecdir,
            systemc_include,
            systemc_lib,
            cc,
            cxx,
        })
    }
}

fn discover_pixi_prefix(project_root: &Path) -> Result<PathBuf, String> {
    if let Some(prefix) = env::var_os("CONDA_PREFIX") {
        let prefix = PathBuf::from(prefix);
        if prefix.is_dir() {
            return Ok(prefix);
        }
        return Err(format!(
            "CONDA_PREFIX is not a directory: {}",
            prefix.display()
        ));
    }

    let prefix = project_root.join(".pixi").join("envs").join("default");
    prefix.is_dir().then_some(prefix).ok_or_else(|| {
        format!(
            "active Pixi prefix is missing at {}; run this through Pixi",
            project_root
                .join(".pixi")
                .join("envs")
                .join("default")
                .display()
        )
    })
}

fn discover_make(project_root: &Path, pixi_prefix: &Path) -> Result<PathBuf, String> {
    if let Some(configured) = env::var_os("MAKE_UNDER_TEST") {
        let configured = PathBuf::from(configured);
        let candidate = if configured.is_absolute() {
            configured
        } else {
            project_root.join(configured)
        };
        if candidate.is_file() {
            return Ok(candidate);
        }
        return Err(format!(
            "MAKE_UNDER_TEST does not point to a file: {}",
            candidate.display()
        ));
    }
    discover_pixi_file(
        pixi_prefix,
        &[
            "Library/usr/bin/make.exe",
            "Library/bin/make.exe",
            "bin/make.exe",
            "bin/make",
        ],
        "make",
    )
}

fn discover_iverilog(project_root: &Path, pixi_prefix: &Path) -> Result<PathBuf, String> {
    if let Some(configured) = env::var_os("IVERILOG_UNDER_TEST") {
        let configured = PathBuf::from(configured);
        let candidate = if configured.is_absolute() {
            configured
        } else {
            project_root.join(configured)
        };
        if candidate.is_file() {
            return Ok(candidate);
        }
        return Err(format!(
            "IVERILOG_UNDER_TEST does not point to a file: {}",
            candidate.display()
        ));
    }
    discover_pixi_file(
        pixi_prefix,
        &[
            "Library/bin/iverilog.exe",
            "bin/iverilog.exe",
            "bin/iverilog",
        ],
        "Icarus Verilog",
    )
}

fn discover_pixi_directory(
    prefix: &Path,
    candidates: &[&str],
    label: &str,
) -> Result<PathBuf, String> {
    candidates
        .iter()
        .map(|candidate| prefix.join(candidate))
        .find(|candidate| candidate.is_dir())
        .ok_or_else(|| {
            format!(
                "{label} directory is missing below Pixi prefix {}; checked {}",
                prefix.display(),
                candidates.join(", ")
            )
        })
}

fn discover_pixi_file(prefix: &Path, candidates: &[&str], label: &str) -> Result<PathBuf, String> {
    candidates
        .iter()
        .map(|candidate| prefix.join(candidate))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| {
            format!(
                "{label} is missing below Pixi prefix {}; checked {}",
                prefix.display(),
                candidates.join(", ")
            )
        })
}

fn discover_optional_companion_tool(
    project_root: &Path,
    bsc: &Path,
    configured: Option<(&str, OsString)>,
    name: &str,
) -> Result<Option<PathBuf>, String> {
    if let Some((environment, configured)) = configured {
        let configured = PathBuf::from(configured);
        let candidate = if configured.is_absolute() {
            configured
        } else {
            project_root.join(configured)
        };
        validate_native_executable(&candidate, environment)?;
        return Ok(Some(candidate));
    }

    let directory = bsc
        .parent()
        .ok_or_else(|| format!("BSC path has no parent directory: {}", bsc.display()))?;
    let candidates = if cfg!(windows) {
        [directory.join(format!("{name}.exe")), directory.join(name)]
    } else {
        [directory.join(name), directory.join(format!("{name}.exe"))]
    };
    for candidate in candidates {
        match fs::symlink_metadata(&candidate) {
            Ok(_) => {
                validate_native_executable(&candidate, "BSC optional companion tool")?;
                return Ok(Some(candidate));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!(
                    "inspect optional BSC companion tool {}: {error}",
                    candidate.display()
                ));
            }
        }
    }
    Ok(None)
}

fn validate_native_executable(path: &Path, label: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "{label} does not point to a file {}: {error}",
            path.display()
        )
    })?;
    if metadata.file_type().is_symlink() || is_reparse_point(&metadata) || !metadata.is_file() {
        return Err(format!(
            "{label} must point to a regular non-link executable file: {}",
            path.display()
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(format!(
                "{label} must point to an executable file: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

fn discover_companion_tool(
    project_root: &Path,
    bsc: &Path,
    environment: &str,
    name: &str,
) -> Result<PathBuf, String> {
    if let Some(configured) = env::var_os(environment) {
        let configured = PathBuf::from(configured);
        let candidate = if configured.is_absolute() {
            configured
        } else {
            project_root.join(configured)
        };
        if candidate.is_file() {
            return Ok(candidate);
        }
        return Err(format!(
            "{environment} does not point to a file: {}",
            candidate.display()
        ));
    }

    let directory = bsc
        .parent()
        .ok_or_else(|| format!("BSC path has no parent directory: {}", bsc.display()))?;
    [
        directory.join(format!("{name}.exe")),
        directory.join(name),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
    .ok_or_else(|| {
        format!(
            "BSC companion tool {name} is missing next to {}; run `pixi run just build` first or set {environment}",
            bsc.display()
        )
    })
}

#[derive(Debug)]
pub struct CommandResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub output: String,
    pub duration: Duration,
}

pub fn current_run_id() -> &'static str {
    static RUN_ID: OnceLock<String> = OnceLock::new();
    RUN_ID.get_or_init(|| {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("{}-{timestamp}", std::process::id())
    })
}

pub fn locate_project_root() -> Result<PathBuf, String> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    for candidate in manifest_dir.ancestors() {
        if candidate
            .join("rust")
            .join("tests")
            .join("Cargo.toml")
            .is_file()
            && candidate
                .join("testsuite")
                .join("bsc.scheduler")
                .join("sat")
                .is_dir()
        {
            return Ok(candidate.to_path_buf());
        }
    }

    Err(format!(
        "could not locate the BSC project root above {}",
        manifest_dir.display()
    ))
}

pub fn run_bsc(
    toolchain: &Toolchain,
    arguments: &[&str],
    cwd: &Path,
    log_path: &Path,
    timeout: Duration,
) -> Result<CommandResult, String> {
    run_bsc_with_options(toolchain, arguments, cwd, log_path, timeout, None)
}

pub fn run_bsc_with_options(
    toolchain: &Toolchain,
    arguments: &[&str],
    cwd: &Path,
    log_path: &Path,
    timeout: Duration,
    options_append: Option<&str>,
) -> Result<CommandResult, String> {
    run_bsc_with_options_and_environment(
        toolchain,
        arguments,
        cwd,
        log_path,
        timeout,
        options_append,
        None,
    )
}

pub fn run_bsc_with_options_prepend(
    toolchain: &Toolchain,
    arguments: &[&str],
    cwd: &Path,
    log_path: &Path,
    timeout: Duration,
    options_prepend: &str,
) -> Result<CommandResult, String> {
    run_command_with_bsc_options(
        toolchain,
        &toolchain.bsc,
        arguments,
        cwd,
        log_path,
        timeout,
        Some(options_prepend),
        None,
        None,
    )
}

pub fn run_bsc_with_options_and_environment(
    toolchain: &Toolchain,
    arguments: &[&str],
    cwd: &Path,
    log_path: &Path,
    timeout: Duration,
    options_append: Option<&str>,
    environment: Option<bsc_test_plan::BscCompileEnvironment>,
) -> Result<CommandResult, String> {
    run_command_with_bsc_options(
        toolchain,
        &toolchain.bsc,
        arguments,
        cwd,
        log_path,
        timeout,
        None,
        options_append,
        environment,
    )
}

pub fn run_command(
    toolchain: &Toolchain,
    program: &Path,
    arguments: &[&str],
    cwd: &Path,
    log_path: &Path,
    timeout: Duration,
) -> Result<CommandResult, String> {
    run_command_with_bsc_options(
        toolchain, program, arguments, cwd, log_path, timeout, None, None, None,
    )
}

fn run_command_with_bsc_options(
    toolchain: &Toolchain,
    program: &Path,
    arguments: &[&str],
    cwd: &Path,
    log_path: &Path,
    timeout: Duration,
    options_prepend: Option<&str>,
    options_append: Option<&str>,
    compile_environment: Option<bsc_test_plan::BscCompileEnvironment>,
) -> Result<CommandResult, String> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| io_error("create command log directory", parent, error))?;
    }

    let mut log = OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(log_path)
        .map_err(|error| io_error("create command log", log_path, error))?;
    writeln!(log, "$ {}", format_command(program.as_os_str(), arguments))
        .and_then(|_| writeln!(log, "cwd: {}\n", cwd.display()))
        .map_err(|error| io_error("write command log header", log_path, error))?;
    log.flush()
        .map_err(|error| io_error("flush command log header", log_path, error))?;
    let output_start = log
        .stream_position()
        .map_err(|error| io_error("record command output start", log_path, error))?;
    let stdout_log = log
        .try_clone()
        .map_err(|error| io_error("clone command stdout log", log_path, error))?;
    let stderr_log = log
        .try_clone()
        .map_err(|error| io_error("clone command stderr log", log_path, error))?;

    let inherited_path = env::var_os("PATH").unwrap_or_default();
    let mut command_paths = pixi_runtime_paths(&toolchain.make);
    for tool in [&toolchain.bsc, &toolchain.make, &toolchain.cxx] {
        if let Some(directory) = tool.parent() {
            command_paths.push(directory.to_path_buf());
        }
    }
    command_paths.extend(env::split_paths(&inherited_path));
    let command_path = env::join_paths(command_paths)
        .map_err(|error| format!("construct command PATH: {error}"))?;

    let command_bluespecdir = if program == toolchain.bluetcl {
        tcl_path(&toolchain.bluespecdir)
    } else {
        bluespecdir_for_program(program, &toolchain.bluespecdir)
    };
    let command_iverilog = shell_path(&toolchain.iverilog);
    let inherited_bsc_options = env::var("BSC_OPTIONS")
        .ok()
        .filter(|inherited| !inherited.trim().is_empty());
    let bsc_options = match (options_prepend, inherited_bsc_options, options_append) {
        (Some(prepend), Some(inherited), None) => Some(format!("{prepend} {inherited}")),
        (Some(prepend), None, None) => Some(prepend.to_owned()),
        (None, Some(inherited), Some(append)) => Some(format!("{inherited} {append}")),
        (None, None, Some(append)) => Some(append.to_owned()),
        (None, _, None) => None,
        (Some(_), _, Some(_)) => unreachable!("BSC_OPTIONS cannot prepend and append together"),
    };
    let started = Instant::now();
    let mut command = Command::new(program);
    command
        .args(arguments)
        .current_dir(cwd)
        .env("PATH", command_path)
        .env("BLUESPECDIR", command_bluespecdir)
        .env("IVERILOG", command_iverilog)
        .env("BSCTEST", "1");
    if let Some(options) = bsc_options {
        command.env("BSC_OPTIONS", options);
    }
    if matches!(
        compile_environment,
        Some(bsc_test_plan::BscCompileEnvironment::GhcrtsM1_2g)
    ) {
        command.env("GHCRTS", "-M1.2G");
    }
    command
        .stdout(Stdio::from(stdout_log))
        .stderr(Stdio::from(stderr_log));
    let mut command = CommandWrap::from(command);
    #[cfg(windows)]
    command.wrap(JobObject);
    #[cfg(unix)]
    command.wrap(ProcessGroup::leader());
    let mut child = command
        .spawn()
        .map_err(|error| io_error("start command", program, error))?;

    let (status, timed_out) = wait_with_timeout(child.as_mut(), timeout)?;
    let duration = started.elapsed();
    let output_end = log
        .metadata()
        .map_err(|error| io_error("inspect command output log", log_path, error))?
        .len();
    let output = if timed_out {
        String::new()
    } else {
        let output_bytes = read_log_range(log_path, output_start, output_end)?;
        String::from_utf8_lossy(&output_bytes).into_owned()
    };

    log.seek(SeekFrom::End(0))
        .and_then(|_| {
            writeln!(log)?;
            writeln!(
                log,
                "exit: {}",
                status
                    .map(describe_status)
                    .unwrap_or_else(|| "timed out".to_owned())
            )?;
            writeln!(log, "duration: {:.3}s", duration.as_secs_f64())?;
            if timed_out {
                writeln!(log, "timeout: {}s", timeout.as_secs())?;
            }
            log.flush()
        })
        .map_err(|error| io_error("write command log footer", log_path, error))?;

    if timed_out {
        return Err(format!(
            "command timed out after {}s; see {}",
            timeout.as_secs(),
            log_path.display()
        ));
    }

    let status = status.expect("non-timeout commands always return an exit status");
    Ok(CommandResult {
        success: status.success(),
        exit_code: portable_exit_code(&status),
        output,
        duration,
    })
}

#[cfg(windows)]
fn pixi_runtime_paths(make: &Path) -> Vec<PathBuf> {
    let Some(library) = make.parent().and_then(Path::parent).and_then(Path::parent) else {
        return Vec::new();
    };
    let runtime = library.join("bin");
    runtime.is_dir().then_some(runtime).into_iter().collect()
}

#[cfg(not(windows))]
fn pixi_runtime_paths(_make: &Path) -> Vec<PathBuf> {
    Vec::new()
}

fn bluespecdir_for_program(program: &Path, bluespecdir: &Path) -> OsString {
    if !cfg!(windows) || program.file_name() != Some(OsStr::new("sh")) {
        return bluespecdir.as_os_str().to_owned();
    }
    shell_path(bluespecdir)
}

fn tcl_path(path: &Path) -> OsString {
    if cfg!(windows) {
        OsString::from(path.to_string_lossy().replace('\\', "/"))
    } else {
        path.as_os_str().to_owned()
    }
}

fn shell_path(path: &Path) -> OsString {
    if !cfg!(windows) {
        return path.as_os_str().to_owned();
    }
    let path = path.to_string_lossy().replace('\\', "/");
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        let drive = (bytes[0] as char).to_ascii_lowercase();
        OsString::from(format!("/{drive}{}", &path[2..]))
    } else {
        OsString::from(path)
    }
}

pub fn normalize_generated_ids(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut normalized = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        let generated_prefix = index + 3 <= bytes.len()
            && &bytes[index..index + 2] == b"__"
            && matches!(bytes[index + 2], b'h' | b'd');
        if generated_prefix {
            let mut end = index + 3;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            let has_digits = end > index + 3;
            let at_identifier_boundary =
                end == bytes.len() || !(bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_');
            if has_digits && at_identifier_boundary {
                normalized.extend_from_slice(&bytes[index..index + 3]);
                normalized.extend_from_slice(b"NNNN");
                index = end;
                continue;
            }
        }

        normalized.push(bytes[index]);
        index += 1;
    }

    String::from_utf8(normalized).expect("normalizing ASCII markers preserves UTF-8")
}

pub fn normalize_sat_solver_names(text: &str) -> String {
    text.replace("_sat-stp", "_sat-SOLVER")
        .replace("_sat-yices", "_sat-SOLVER")
        .replace("_sat-z3", "_sat-SOLVER")
}

pub fn normalize_golden_output(text: &str) -> String {
    let normalized_newlines = text.replace("\r\n", "\n").replace('\r', "\n");
    let normalized_newlines = normalize_windows_scientific_exponents(&normalized_newlines);
    let mut filtered = String::with_capacity(normalized_newlines.len());
    for line in normalized_newlines.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if !line.contains("SystemC")
            && !line.contains("dumpfile parameter")
            && !trimmed.starts_with("compiling ./")
        {
            filtered.push_str(line);
        }
    }
    let normalized = normalize_diff_b_text(&filtered);
    normalized
        .strip_suffix('\n')
        .unwrap_or(&normalized)
        .to_owned()
}

fn normalize_windows_scientific_exponents(text: &str) -> String {
    static WINDOWS_EXPONENT: OnceLock<regex::Regex> = OnceLock::new();
    WINDOWS_EXPONENT
        .get_or_init(|| {
            regex::Regex::new(r"([0-9][eE][+-])0([0-9]{2})([^0-9]|$)")
                .expect("Windows scientific exponent regex is valid")
        })
        .replace_all(text, "${1}${2}${3}")
        .into_owned()
}

pub fn normalize_diff_b_text(text: &str) -> String {
    let newlines_normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let has_final_newline = newlines_normalized.ends_with('\n');
    let mut lines: Vec<&str> = newlines_normalized.split('\n').collect();
    if has_final_newline {
        lines.pop();
    }

    let normalized_lines: Vec<String> = lines.into_iter().map(normalize_diff_b_line).collect();
    let mut normalized = normalized_lines.join("\n");
    if has_final_newline {
        normalized.push('\n');
    }
    normalized
}

fn normalize_diff_b_line(line: &str) -> String {
    let mut normalized = String::with_capacity(line.len());
    let mut in_horizontal_whitespace = false;
    for character in line.chars() {
        if matches!(character, ' ' | '\t') {
            if !in_horizontal_whitespace {
                normalized.push(' ');
                in_horizontal_whitespace = true;
            }
        } else {
            normalized.push(character);
            in_horizontal_whitespace = false;
        }
    }
    while normalized.ends_with(' ') {
        normalized.pop();
    }
    normalized
}

pub(crate) fn reset_directory(path: &Path) -> Result<(), String> {
    match fs::remove_dir_all(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error("remove old test directory", path, error)),
    }
    fs::create_dir_all(path).map_err(|error| io_error("create test directory", path, error))
}

fn read_log_range(log_path: &Path, start: u64, end: u64) -> Result<Vec<u8>, String> {
    let length = end
        .checked_sub(start)
        .ok_or_else(|| format!("invalid command output range {start}..{end}"))?;
    let mut log = File::open(log_path)
        .map_err(|error| io_error("open command output log", log_path, error))?;
    log.seek(SeekFrom::Start(start))
        .map_err(|error| io_error("seek command output log", log_path, error))?;
    let mut output = Vec::new();
    log.take(length)
        .read_to_end(&mut output)
        .map_err(|error| io_error("read command output log", log_path, error))?;
    Ok(output)
}

fn wait_with_timeout(
    child: &mut dyn ChildWrapper,
    timeout: Duration,
) -> Result<(Option<ExitStatus>, bool), String> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok((Some(status), false)),
            Ok(None) if started.elapsed() >= timeout => {
                child
                    .start_kill()
                    .map_err(|error| format!("terminate command process tree: {error}"))?;
                return Ok((None, true));
            }
            Ok(None) => {
                let remaining = timeout.saturating_sub(started.elapsed());
                thread::sleep(std::cmp::min(Duration::from_millis(50), remaining));
            }
            Err(error) => return Err(format!("poll command process tree: {error}")),
        }
    }
}

fn format_command(executable: &OsStr, arguments: &[&str]) -> String {
    std::iter::once(executable.to_string_lossy().into_owned())
        .chain(arguments.iter().map(|argument| (*argument).to_owned()))
        .map(|argument| quote_argument(&argument))
        .collect::<Vec<_>>()
        .join(" ")
}

fn quote_argument(argument: &str) -> String {
    if argument.is_empty()
        || argument
            .chars()
            .any(|character| character.is_whitespace() || matches!(character, '"' | '\''))
    {
        format!("{argument:?}")
    } else {
        argument.to_owned()
    }
}

fn portable_exit_code(status: &ExitStatus) -> Option<i32> {
    #[cfg(unix)]
    {
        status
            .code()
            .or_else(|| status.signal().map(|signal| 128 + signal))
    }
    #[cfg(not(unix))]
    {
        status.code()
    }
}

fn describe_status(status: ExitStatus) -> String {
    status.code().map_or_else(
        || "terminated by signal".to_owned(),
        |code| code.to_string(),
    )
}

fn io_error(action: &str, path: &Path, error: io::Error) -> String {
    format!("{action} {}: {error}", path.display())
}

#[derive(Clone, Copy)]
enum DiffOp<'a> {
    Equal(&'a str),
    Remove(&'a str),
    Add(&'a str),
}

pub fn readable_diff(
    expected: &str,
    actual: &str,
    expected_label: &str,
    actual_label: &str,
) -> String {
    let expected_lines: Vec<&str> = expected.split_inclusive('\n').collect();
    let actual_lines: Vec<&str> = actual.split_inclusive('\n').collect();
    let column_count = actual_lines.len() + 1;
    let mut lcs = vec![0usize; (expected_lines.len() + 1) * column_count];

    for expected_index in (0..expected_lines.len()).rev() {
        for actual_index in (0..actual_lines.len()).rev() {
            let cell = expected_index * column_count + actual_index;
            lcs[cell] = if expected_lines[expected_index] == actual_lines[actual_index] {
                lcs[(expected_index + 1) * column_count + actual_index + 1] + 1
            } else {
                std::cmp::max(
                    lcs[(expected_index + 1) * column_count + actual_index],
                    lcs[expected_index * column_count + actual_index + 1],
                )
            };
        }
    }

    let mut operations = Vec::new();
    let (mut expected_index, mut actual_index) = (0, 0);
    while expected_index < expected_lines.len() && actual_index < actual_lines.len() {
        if expected_lines[expected_index] == actual_lines[actual_index] {
            operations.push(DiffOp::Equal(expected_lines[expected_index]));
            expected_index += 1;
            actual_index += 1;
        } else if lcs[(expected_index + 1) * column_count + actual_index]
            >= lcs[expected_index * column_count + actual_index + 1]
        {
            operations.push(DiffOp::Remove(expected_lines[expected_index]));
            expected_index += 1;
        } else {
            operations.push(DiffOp::Add(actual_lines[actual_index]));
            actual_index += 1;
        }
    }
    operations.extend(
        expected_lines[expected_index..]
            .iter()
            .copied()
            .map(DiffOp::Remove),
    );
    operations.extend(
        actual_lines[actual_index..]
            .iter()
            .copied()
            .map(DiffOp::Add),
    );

    let mut intervals = Vec::<(usize, usize)>::new();
    for changed_index in operations
        .iter()
        .enumerate()
        .filter_map(|(index, operation)| match operation {
            DiffOp::Equal(_) => None,
            DiffOp::Remove(_) | DiffOp::Add(_) => Some(index),
        })
    {
        let start = changed_index.saturating_sub(3);
        let end = std::cmp::min(changed_index + 4, operations.len());
        match intervals.last_mut() {
            Some((_, previous_end)) if start <= *previous_end => {
                *previous_end = std::cmp::max(*previous_end, end);
            }
            _ => intervals.push((start, end)),
        }
    }

    let mut diff = format!("--- {expected_label}\n+++ {actual_label}\n");
    for (start, end) in intervals {
        let (old_start, new_start) = line_numbers_before(&operations, start);
        let old_count = operations[start..end]
            .iter()
            .filter(|operation| !matches!(operation, DiffOp::Add(_)))
            .count();
        let new_count = operations[start..end]
            .iter()
            .filter(|operation| !matches!(operation, DiffOp::Remove(_)))
            .count();
        diff.push_str(&format!(
            "@@ -{old_start},{old_count} +{new_start},{new_count} @@\n"
        ));
        for operation in &operations[start..end] {
            match operation {
                DiffOp::Equal(line) => push_diff_line(&mut diff, ' ', line),
                DiffOp::Remove(line) => push_diff_line(&mut diff, '-', line),
                DiffOp::Add(line) => push_diff_line(&mut diff, '+', line),
            }
        }
    }
    diff
}

fn line_numbers_before(operations: &[DiffOp<'_>], end: usize) -> (usize, usize) {
    let (mut old_line, mut new_line) = (1, 1);
    for operation in &operations[..end] {
        match operation {
            DiffOp::Equal(_) => {
                old_line += 1;
                new_line += 1;
            }
            DiffOp::Remove(_) => old_line += 1,
            DiffOp::Add(_) => new_line += 1,
        }
    }
    (old_line, new_line)
}

fn push_diff_line(diff: &mut String, prefix: char, line: &str) {
    diff.push(prefix);
    diff.push_str(line);
    if !line.ends_with('\n') {
        diff.push('\n');
        diff.push_str("\\ No newline at end of file\n");
    }
}

#[cfg(test)]
mod tests {
    use super::{
        bluespecdir_for_program, current_run_id, discover_optional_companion_tool,
        normalize_diff_b_text, normalize_generated_ids, normalize_sat_solver_names, read_log_range,
        run_command, tcl_path, Toolchain,
    };
    use std::ffi::OsString;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn normalization_generated_ids() {
        assert_eq!(
            normalize_generated_ids("a__h12 x__d34; __h5_ __d6x __h7\n"),
            "a__hNNNN x__dNNNN; __h5_ __d6x __hNNNN\n"
        );
    }

    #[test]
    fn normalization_sat_solver_suffixes() {
        assert_eq!(
            normalize_sat_solver_names("Foo_sat-z3 Foo_sat-yices Foo_sat-stp"),
            "Foo_sat-SOLVER Foo_sat-SOLVER Foo_sat-SOLVER"
        );
    }

    #[test]
    fn normalization_crlf_and_diff_b_whitespace() {
        assert_eq!(
            normalize_diff_b_text("alpha  \t beta \r\ntrail\t \rsolo\tvalue\r"),
            "alpha beta\ntrail\nsolo value\n"
        );
    }

    #[cfg(windows)]
    #[test]
    fn msys_shell_receives_a_posix_bluespecdir() {
        assert_eq!(
            tcl_path(Path::new(r"D:\projects\bsc\inst\lib")),
            "D:/projects/bsc/inst/lib"
        );
        assert_eq!(
            bluespecdir_for_program(Path::new("sh"), Path::new(r"D:\projects\bsc\inst\lib")),
            "/d/projects/bsc/inst/lib"
        );
        assert_eq!(
            bluespecdir_for_program(Path::new("bsc.exe"), Path::new(r"D:\projects\bsc\inst\lib")),
            r"D:\projects\bsc\inst\lib"
        );
    }

    #[test]
    fn run_id_contains_pid_and_timestamp() {
        let run_id = current_run_id();
        let (pid, timestamp) = run_id.split_once('-').expect("run id separator");
        assert_eq!(pid, std::process::id().to_string());
        assert!(timestamp.parse::<u128>().is_ok());
        assert_eq!(run_id, current_run_id());
    }

    #[test]
    fn log_range_reads_only_requested_bytes() {
        let temp = TestDirectory::new("log-range");
        let log_path = temp.path.join("command.log");
        fs::write(&log_path, b"header\noutput\nfooter\n").expect("write command log");

        assert_eq!(
            read_log_range(&log_path, 7, 14).expect("read output range"),
            b"output\n"
        );
        assert!(read_log_range(&log_path, 14, 7)
            .expect_err("reversed ranges are invalid")
            .contains("invalid command output range 14..7"));
    }

    #[test]
    fn command_start_failure_keeps_command_log_header() {
        let temp = TestDirectory::new("command-start-failure");
        let log_path = temp.path.join("command.log");
        let program = temp.path.join("missing-command.exe");
        let toolchain = test_toolchain(&temp.path, &program);

        let error = run_command(
            &toolchain,
            &program,
            &[],
            &temp.path,
            &log_path,
            Duration::from_secs(5),
        )
        .expect_err("missing command must fail to start");

        assert!(error.contains("start command"));
        assert!(error.contains("missing-command.exe"));
        let log = fs::read_to_string(&log_path).expect("read command log");
        assert!(log.contains("$"));
        assert!(log.contains("missing-command.exe"));
        assert!(log.contains(&format!("cwd: {}", temp.path.display())));
        assert!(!log.contains("exit:"));
    }

    #[test]
    fn command_output_without_trailing_newline_excludes_log_footer() {
        let temp = TestDirectory::new("command-output-no-newline");
        let log_path = temp.path.join("command.log");
        let (program, arguments) = no_newline_command();
        let toolchain = test_toolchain(&temp.path, &program);

        let result = run_command(
            &toolchain,
            &program,
            &arguments,
            &temp.path,
            &log_path,
            Duration::from_secs(5),
        )
        .expect("test command should run");

        assert!(result.success);
        assert_eq!(result.output, "output without newline");
        let log = fs::read_to_string(&log_path).expect("read command log");
        assert!(log.contains("output without newline\nexit: 0"));
    }

    #[test]
    fn command_output_is_appended_to_log_and_read_by_range() {
        let temp = TestDirectory::new("command-output");
        let log_path = temp.path.join("command.log");
        let (program, arguments) = test_command();
        let toolchain = test_toolchain(&temp.path, &program);

        let result = run_command(
            &toolchain,
            &program,
            &arguments,
            &temp.path,
            &log_path,
            Duration::from_secs(5),
        )
        .expect("test command should run");

        assert!(result.success);
        assert!(result.output.contains("stdout line"));
        assert!(result.output.contains("stderr line"));
        let log = fs::read_to_string(&log_path).expect("read command log");
        assert!(log.contains("stdout line"));
        assert!(log.contains("stderr line"));
        assert!(log.contains("exit: 0"));
        assert!(!result.output.contains("exit: 0"));
    }

    #[test]
    fn failed_command_preserves_merged_output_and_exit_code() {
        let temp = TestDirectory::new("command-failure");
        let log_path = temp.path.join("command.log");
        let (program, arguments) = failing_command();
        let toolchain = test_toolchain(&temp.path, &program);

        let result = run_command(
            &toolchain,
            &program,
            &arguments,
            &temp.path,
            &log_path,
            Duration::from_secs(5),
        )
        .expect("test command should complete");

        assert!(!result.success);
        assert_eq!(result.exit_code, Some(7));
        assert!(result.output.contains("stdout failure"));
        assert!(result.output.contains("stderr failure"));
        let log = fs::read_to_string(&log_path).expect("read command log");
        assert!(log.contains("stdout failure"));
        assert!(log.contains("stderr failure"));
        assert!(log.contains("exit: 7"));
    }

    #[test]
    fn timed_out_command_keeps_merged_log_and_reports_timeout() {
        let temp = TestDirectory::new("command-timeout");
        let log_path = temp.path.join("command.log");
        let (program, arguments) = timeout_command();
        let toolchain = test_toolchain(&temp.path, &program);

        let error = run_command(
            &toolchain,
            &program,
            &arguments,
            &temp.path,
            &log_path,
            Duration::from_millis(100),
        )
        .expect_err("test command should time out");

        assert!(error.contains("command timed out after 0s"));
        let log = fs::read_to_string(&log_path).expect("read command log");
        assert!(log.contains("before timeout"));
        assert!(log.contains("exit: timed out"));
        assert!(log.contains("timeout: 0s"));
    }

    #[cfg(windows)]
    fn test_command() -> (PathBuf, Vec<&'static str>) {
        (
            PathBuf::from("cmd.exe"),
            vec!["/C", "echo stdout line & echo stderr line 1>&2"],
        )
    }

    #[cfg(not(windows))]
    fn test_command() -> (PathBuf, Vec<&'static str>) {
        (
            PathBuf::from("sh"),
            vec!["-c", "printf 'stdout line\\n'; printf 'stderr line\\n' >&2"],
        )
    }

    #[cfg(not(windows))]
    fn no_newline_command() -> (PathBuf, Vec<&'static str>) {
        (
            PathBuf::from("sh"),
            vec!["-c", "printf 'output without newline'"],
        )
    }

    #[cfg(windows)]
    fn no_newline_command() -> (PathBuf, Vec<&'static str>) {
        (
            PathBuf::from("powershell.exe"),
            vec![
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "[Console]::Out.Write('output without newline')",
            ],
        )
    }

    #[cfg(windows)]
    fn failing_command() -> (PathBuf, Vec<&'static str>) {
        (
            PathBuf::from("cmd.exe"),
            vec![
                "/C",
                "echo stdout failure & echo stderr failure 1>&2 & exit /B 7",
            ],
        )
    }

    #[cfg(not(windows))]
    fn failing_command() -> (PathBuf, Vec<&'static str>) {
        (
            PathBuf::from("sh"),
            vec![
                "-c",
                "printf 'stdout failure\\n'; printf 'stderr failure\\n' >&2; exit 7",
            ],
        )
    }

    #[cfg(windows)]
    fn timeout_command() -> (PathBuf, Vec<&'static str>) {
        (
            PathBuf::from("cmd.exe"),
            vec!["/C", "echo before timeout & ping -n 4 127.0.0.1 > NUL"],
        )
    }

    #[cfg(not(windows))]
    fn timeout_command() -> (PathBuf, Vec<&'static str>) {
        (
            PathBuf::from("sh"),
            vec!["-c", "printf 'before timeout\\n'; sleep 3"],
        )
    }

    #[test]
    fn optional_showrules_discovery_distinguishes_missing_present_and_invalid_overrides() {
        let temp = TestDirectory::new("showrules-discovery");
        let tools = temp.path.join("tools");
        fs::create_dir_all(&tools).unwrap();
        let bsc = tools.join(if cfg!(windows) { "bsc.exe" } else { "bsc" });
        make_native_executable(&bsc);

        assert_eq!(
            discover_optional_companion_tool(&temp.path, &bsc, None, "showrules").unwrap(),
            None
        );

        let native = tools.join(if cfg!(windows) {
            "showrules.exe"
        } else {
            "showrules"
        });
        make_native_executable(&native);
        assert_eq!(
            discover_optional_companion_tool(&temp.path, &bsc, None, "showrules").unwrap(),
            Some(native.clone())
        );

        let explicit = temp.path.join("explicit-showrules");
        make_native_executable(&explicit);
        assert_eq!(
            discover_optional_companion_tool(
                &temp.path,
                &bsc,
                Some(("SHOWRULES_UNDER_TEST", explicit.clone().into_os_string())),
                "showrules",
            )
            .unwrap(),
            Some(explicit)
        );
        let error = discover_optional_companion_tool(
            &temp.path,
            &bsc,
            Some(("SHOWRULES", OsString::from("missing-showrules"))),
            "showrules",
        )
        .unwrap_err();
        assert!(error.contains("SHOWRULES"));
        assert!(error.contains("missing-showrules"));
    }

    #[cfg(unix)]
    #[test]
    fn optional_showrules_discovery_rejects_symlink_overrides() {
        use std::os::unix::fs::symlink;

        let temp = TestDirectory::new("showrules-discovery-symlink");
        let target = temp.path.join("showrules-real");
        let link = temp.path.join("showrules-link");
        make_native_executable(&target);
        symlink(&target, &link).unwrap();
        let error = discover_optional_companion_tool(
            &temp.path,
            &target,
            Some(("SHOWRULES_UNDER_TEST", link.into_os_string())),
            "showrules",
        )
        .unwrap_err();
        assert!(error.contains("regular non-link executable"));
    }

    fn make_native_executable(path: &Path) {
        fs::write(path, b"fake executable").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    fn test_toolchain(root: &Path, program: &Path) -> Toolchain {
        Toolchain {
            project_root: root.to_path_buf(),
            bsc: program.to_path_buf(),
            bluetcl: program.to_path_buf(),
            bsc2bsv: program.to_path_buf(),
            dumpbo: PathBuf::new(),
            dumpba: PathBuf::new(),
            vcdcheck: program.to_path_buf(),
            showrules: Some(program.to_path_buf()),
            make: program.to_path_buf(),
            iverilog: program.to_path_buf(),
            bluespecdir: root.join("lib"),
            systemc_include: PathBuf::new(),
            systemc_lib: PathBuf::new(),
            cc: program.to_path_buf(),
            cxx: PathBuf::new(),
        }
    }

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock must be after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "bsc-rust-tests-{label}-{}-{timestamp}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("create temporary test directory");
            Self { path }
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
