use std::path::{Path, PathBuf};

pub(crate) struct BluesimInvocation {
    pub program: PathBuf,
    pub arguments: Vec<String>,
}

pub(crate) fn invocation(executable: &Path, top: &str, arguments: &[String]) -> BluesimInvocation {
    invocation_for_platform(executable, top, arguments, cfg!(windows))
}

fn invocation_for_platform(
    executable: &Path,
    top: &str,
    arguments: &[String],
    windows: bool,
) -> BluesimInvocation {
    if windows
        && !executable
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        let mut launched_arguments = Vec::with_capacity(arguments.len() + 1);
        launched_arguments.push(top.to_owned());
        launched_arguments.extend_from_slice(arguments);
        BluesimInvocation {
            program: PathBuf::from("sh"),
            arguments: launched_arguments,
        }
    } else {
        BluesimInvocation {
            program: executable.to_owned(),
            arguments: arguments.to_owned(),
        }
    }
}

pub(crate) fn resolve_executable(work_dir: &Path, top: &str) -> Result<PathBuf, String> {
    let executable = work_dir.join(top);
    if executable.is_file() {
        return Ok(executable);
    }
    if cfg!(windows) {
        let executable = work_dir.join(format!("{top}.exe"));
        if executable.is_file() {
            return Ok(executable);
        }
    }
    Err(format!(
        "BSC did not link Bluesim executable {}",
        work_dir.join(top).display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_extensionless_launcher_runs_through_sh() {
        let invocation = invocation_for_platform(
            Path::new("work/mkTestbench"),
            "mkTestbench",
            &["-V".to_owned(), "trace.vcd".to_owned()],
            true,
        );
        assert_eq!(invocation.program, Path::new("sh"));
        assert_eq!(invocation.arguments, ["mkTestbench", "-V", "trace.vcd"]);
    }

    #[test]
    fn windows_cexe_launcher_runs_through_sh() {
        let invocation = invocation_for_platform(
            Path::new("work/mkTestbench.cexe"),
            "mkTestbench.cexe",
            &["-m".to_owned(), "500".to_owned()],
            true,
        );
        assert_eq!(invocation.program, Path::new("sh"));
        assert_eq!(invocation.arguments, ["mkTestbench.cexe", "-m", "500"]);
    }

    #[test]
    fn native_executable_is_invoked_directly() {
        let executable = Path::new("work/mkTestbench.exe");
        let invocation = invocation_for_platform(
            executable,
            "mkTestbench",
            &["-m".to_owned(), "500".to_owned()],
            true,
        );
        assert_eq!(invocation.program, executable);
        assert_eq!(invocation.arguments, ["-m", "500"]);
    }
}
