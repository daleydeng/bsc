use crate::cache::{BscResultCache, ResultCacheLookup};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::OnceLock;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub mod alignment;
pub(crate) mod cache;
pub mod upstream;

pub const BSC_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, Clone)]
pub struct Toolchain {
    pub project_root: PathBuf,
    pub bsc: PathBuf,
    pub bluespecdir: PathBuf,
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
        let bluespecdir = project_root.join("inst").join("lib");
        if !bluespecdir.is_dir() {
            return Err(format!(
                "BSC library directory is missing at {}; run `pixi run just build` first",
                bluespecdir.display()
            ));
        }

        Ok(Self {
            project_root,
            bsc,
            bluespecdir,
        })
    }
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
        if candidate.join("rust-tests").join("Cargo.toml").is_file()
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
    run_command(toolchain, &toolchain.bsc, arguments, cwd, log_path, timeout)
}

pub fn run_command(
    toolchain: &Toolchain,
    program: &Path,
    arguments: &[&str],
    cwd: &Path,
    log_path: &Path,
    timeout: Duration,
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
        .map_err(|error| io_error("seek command log", log_path, error))?;

    let stdout_log = log
        .try_clone()
        .map_err(|error| io_error("clone command stdout log", log_path, error))?;
    let stderr_log = log
        .try_clone()
        .map_err(|error| io_error("clone command stderr log", log_path, error))?;

    let inherited_path = env::var_os("PATH").unwrap_or_default();
    let mut command_paths = Vec::new();
    if let Some(bsc_dir) = toolchain.bsc.parent() {
        command_paths.push(bsc_dir.to_path_buf());
    }
    command_paths.extend(env::split_paths(&inherited_path));
    let command_path = env::join_paths(command_paths)
        .map_err(|error| format!("construct command PATH: {error}"))?;

    let command_bluespecdir = bluespecdir_for_program(program, &toolchain.bluespecdir);
    let started = Instant::now();
    let mut child = Command::new(program)
        .args(arguments)
        .current_dir(cwd)
        .env("PATH", command_path)
        .env("BLUESPECDIR", command_bluespecdir)
        .env("BSCTEST", "1")
        .stdout(Stdio::from(stdout_log))
        .stderr(Stdio::from(stderr_log))
        .spawn()
        .map_err(|error| io_error("start command", program, error))?;

    let (status, timed_out) = wait_with_timeout(&mut child, timeout)?;
    let duration = started.elapsed();
    let output_bytes = read_log_output(log_path, output_start)?;
    let output = String::from_utf8_lossy(&output_bytes).into_owned();

    log.seek(SeekFrom::End(0))
        .and_then(|_| {
            writeln!(log)?;
            writeln!(log, "exit: {}", describe_status(status))?;
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

    Ok(CommandResult {
        success: status.success(),
        exit_code: status.code(),
        output,
        duration,
    })
}

fn bluespecdir_for_program(program: &Path, bluespecdir: &Path) -> OsString {
    if !cfg!(windows) || program.file_name() != Some(OsStr::new("sh")) {
        return bluespecdir.as_os_str().to_owned();
    }

    let path = bluespecdir.to_string_lossy().replace('\\', "/");
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        let drive = (bytes[0] as char).to_ascii_lowercase();
        OsString::from(format!("/{drive}{}", &path[2..]))
    } else {
        OsString::from(path)
    }
}

pub fn run_scheduler_sat_case(case: &str) -> Result<(), String> {
    if case.is_empty()
        || !case
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(format!("invalid scheduler SAT case name: {case:?}"));
    }

    let toolchain = Toolchain::discover()?;
    let source_dir = toolchain
        .project_root
        .join("testsuite")
        .join("bsc.scheduler")
        .join("sat");
    let work_dir = toolchain
        .project_root
        .join(".pixi")
        .join("tmp")
        .join("rust-test-work")
        .join("scheduler-sat")
        .join(current_run_id())
        .join(case);
    let artifact_dir = toolchain
        .project_root
        .join(".pixi")
        .join("tmp")
        .join("rust-test-artifacts")
        .join("scheduler-sat")
        .join(current_run_id())
        .join(case);
    reset_directory(&work_dir)?;
    reset_directory(&artifact_dir)?;

    let renamed_stem = format!("{case}_sat-z3");
    let source_file_name = format!("{case}.bsv");
    let source = source_dir.join(&source_file_name);
    let staged_file_name = format!("{renamed_stem}.bsv");
    let staged_source = work_dir.join(&staged_file_name);
    fs::copy(&source, &staged_source).map_err(|error| {
        format!(
            "copy {} to {}: {error}",
            source.display(),
            staged_source.display()
        )
    })?;

    let compile_log = artifact_dir.join("bsc-schedule.log");
    let arguments = [
        "-sat-z3",
        "-no-show-timestamps",
        "-no-show-version",
        "-u",
        "-resource-simple",
        "-show-schedule",
        "-dschedule",
        "-dresources",
        "-dvschedinfo",
        "-verilog",
        staged_file_name.as_str(),
    ];
    let result_cache = scheduler_result_cache(&toolchain);
    let (result, cache_key) = match result_cache.lookup(
        &source_dir,
        &[source_file_name.as_str()],
        &arguments,
        &work_dir,
        &compile_log,
    ) {
        Ok(ResultCacheLookup::Hit(result)) => (result, None),
        Ok(ResultCacheLookup::Miss(key)) => (
            run_bsc(&toolchain, &arguments, &work_dir, &compile_log, BSC_TIMEOUT)?,
            Some(key),
        ),
        Ok(ResultCacheLookup::Disabled) => (
            run_bsc(&toolchain, &arguments, &work_dir, &compile_log, BSC_TIMEOUT)?,
            None,
        ),
        Err(error) => {
            eprintln!("warning: BSC result cache lookup failed for scheduler case {case}: {error}");
            (
                run_bsc(&toolchain, &arguments, &work_dir, &compile_log, BSC_TIMEOUT)?,
                None,
            )
        }
    };
    if !result.success {
        let exit = result.exit_code.map_or_else(
            || "terminated by signal".to_owned(),
            |code| code.to_string(),
        );
        return Err(format!(
            "BSC exited with {exit} for {case}; see {}",
            compile_log.display()
        ));
    }

    let object_file = work_dir.join(format!("{renamed_stem}.bo"));
    if !object_file.is_file() {
        return Err(format!(
            "BSC succeeded but did not create {}; see {}",
            object_file.display(),
            compile_log.display()
        ));
    }

    let expected_path = source_dir.join(format!("{case}_sat-yices.bsv.bsc-sched-out.expected"));
    assert_scheduler_output_matches(
        &result.output,
        &expected_path,
        &artifact_dir.join("schedule.diff"),
    )?;

    if let Some(key) = cache_key {
        if let Err(error) = result_cache.store(&key, &work_dir, &result) {
            eprintln!("warning: BSC result cache store failed for scheduler case {case}: {error}");
        }
    }

    Ok(())
}

fn scheduler_result_cache(toolchain: &Toolchain) -> &'static BscResultCache {
    static CACHE: OnceLock<BscResultCache> = OnceLock::new();
    CACHE.get_or_init(|| match BscResultCache::new(toolchain) {
        Ok(cache) => cache,
        Err(error) => {
            eprintln!(
                "warning: scheduler BSC result cache initialization failed; continuing uncached: {error}"
            );
            BscResultCache::disabled(toolchain)
        }
    })
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

pub fn assert_scheduler_output_matches(
    actual: &str,
    expected_path: &Path,
    diff_path: &Path,
) -> Result<(), String> {
    let expected = fs::read_to_string(expected_path)
        .map_err(|error| io_error("read expected scheduler output", expected_path, error))?;
    let actual = normalize_scheduler_output(actual);
    let expected = normalize_scheduler_output(&expected);

    if actual == expected {
        match fs::remove_file(diff_path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error("remove stale scheduler diff", diff_path, error)),
        }
        return Ok(());
    }

    if let Some(parent) = diff_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| io_error("create scheduler diff directory", parent, error))?;
    }
    let diff = readable_diff(
        &expected,
        &actual,
        &expected_path.display().to_string(),
        "actual BSC output",
    );
    fs::write(diff_path, diff)
        .map_err(|error| io_error("write scheduler output diff", diff_path, error))?;
    Err(format!(
        "output differs from {}; see {}",
        expected_path.display(),
        diff_path.display()
    ))
}

fn normalize_scheduler_output(text: &str) -> String {
    normalize_diff_b_text(&normalize_sat_solver_names(&normalize_generated_ids(text)))
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

fn reset_directory(path: &Path) -> Result<(), String> {
    match fs::remove_dir_all(path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error("remove old test directory", path, error)),
    }
    fs::create_dir_all(path).map_err(|error| io_error("create test directory", path, error))
}

fn wait_with_timeout(child: &mut Child, timeout: Duration) -> Result<(ExitStatus, bool), String> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok((status, false)),
            Ok(None) if started.elapsed() >= timeout => {
                terminate_process_tree(child);
                let status = child
                    .wait()
                    .map_err(|error| format!("wait for BSC after timeout: {error}"))?;
                return Ok((status, true));
            }
            Ok(None) => {
                let remaining = timeout.saturating_sub(started.elapsed());
                thread::sleep(std::cmp::min(Duration::from_millis(50), remaining));
            }
            Err(error) => return Err(format!("poll BSC process: {error}")),
        }
    }
}

#[cfg(windows)]
fn terminate_process_tree(child: &mut Child) {
    let taskkill_succeeded = Command::new("taskkill.exe")
        .args(["/PID", &child.id().to_string(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if !taskkill_succeeded {
        let _ = child.kill();
    }
}

#[cfg(not(windows))]
fn terminate_process_tree(child: &mut Child) {
    let _ = child.kill();
}

fn read_log_output(path: &Path, offset: u64) -> Result<Vec<u8>, String> {
    let mut file = File::open(path).map_err(|error| io_error("open BSC log", path, error))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| io_error("seek to BSC output", path, error))?;
    let mut output = Vec::new();
    file.read_to_end(&mut output)
        .map_err(|error| io_error("read BSC output", path, error))?;
    Ok(output)
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
        bluespecdir_for_program, current_run_id, normalize_diff_b_text, normalize_generated_ids,
        normalize_sat_solver_names,
    };
    use std::path::Path;

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
}
