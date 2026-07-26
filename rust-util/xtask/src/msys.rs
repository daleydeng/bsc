use std::{
    ffi::OsStr,
    fs::{self, OpenOptions},
    io::{self, Write},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{bail, ensure, Context, Result};

use crate::environment::PreparedEnvironment;

const PREAMBLE: &str = r#"export PATH="$(cygpath -u "$CONDA_PREFIX")/Library/bin:$PATH"
if [ -n "${BSC_OSS_CAD_SUITE_ROOT:-}" ]; then
    preferred_bin="$(cygpath -u "$BSC_PIXI_PREFERRED_BIN")"
    oss_cad_root="$(cygpath -u "$BSC_OSS_CAD_SUITE_ROOT")"
    export PATH="$preferred_bin:$oss_cad_root/bin:$oss_cad_root/lib:$PATH"
fi
export SSL_CERT_FILE="$(cygpath -u "$CONDA_PREFIX")/Library/ssl/cacert.pem"
export GIT_SSL_CAINFO="$SSL_CERT_FILE"
export CURL_CA_BUNDLE="$SSL_CERT_FILE"
"#;

const DOCTOR_COMMAND: &str = r#"set -u
failed=0
for tool in bash make git diff gcc g++ ccache pkg-config tclsh iverilog vvp ghc ghc-pkg cabal rustc cargo z3; do
    if command -v "$tool" >/dev/null 2>&1; then
        printf '%-12s %s\n' "$tool" "$(command -v "$tool" | tr -d '\r')"
    else
        printf '%-12s MISSING\n' "$tool"
        failed=1
    fi
done
printf '%-12s %s\n' OSTYPE "$(./platform.sh ostype | tr -d '\r')"
printf '%-12s %s\n' MACHTYPE "$(./platform.sh machtype | tr -d '\r')"
printf '%-12s %s\n' BUILD_JOBS "$BSC_BUILD_JOBS"
printf '%-12s %s\n' OSS_CAD "$BSC_OSS_CAD_SUITE_ROOT"
iverilog -V 2>&1 | sed -n '1p' || true
ghc --version 2>/dev/null || true
cabal --numeric-version 2>/dev/null || true
rustc --version 2>/dev/null || true
cargo --version 2>/dev/null || true
z3 -version 2>/dev/null || true
exit "$failed"
"#;

pub fn doctor(env: &PreparedEnvironment) -> Result<()> {
    invoke_msys(env, DOCTOR_COMMAND, &[])
}

pub fn build(env: &PreparedEnvironment) -> Result<()> {
    let ghc_temp = env.root.join(".pixi/tmp/ghc");
    fs::create_dir_all(&ghc_temp)
        .with_context(|| format!("could not create {}", ghc_temp.display()))?;

    println!(
        "Building with {} parallel jobs (set BSC_JOBS to override).",
        env.jobs
    );
    let command = format!(
        "make -j{0} GHCJOBS={0} install-src && make -C src/comp -j1 GHCJOBS={0} PREFIX=../../inst install-extra",
        env.jobs
    );
    invoke_msys(
        env,
        &command,
        &[
            ("TEMP", ghc_temp.as_os_str()),
            ("TMP", ghc_temp.as_os_str()),
        ],
    )
}

pub fn smoke(env: &PreparedEnvironment) -> Result<()> {
    invoke_msys(env, "make check-smoke", &[])
}

pub fn clean(env: &PreparedEnvironment) -> Result<()> {
    invoke_msys(env, "make full_clean", &[])
}

pub fn shell(env: &PreparedEnvironment) -> Result<()> {
    run_bash(
        env,
        &[
            OsStr::new("--noprofile"),
            OsStr::new("--norc"),
            OsStr::new("-i"),
        ],
        &[],
    )
}

fn invoke_msys(
    env: &PreparedEnvironment,
    command: &str,
    child_environment: &[(&str, &OsStr)],
) -> Result<()> {
    let temp_dir = env.root.join(".pixi/tmp");
    fs::create_dir_all(&temp_dir)
        .with_context(|| format!("could not create {}", temp_dir.display()))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before the Unix epoch")?
        .as_nanos();
    let name = format!("xtask-{}-{timestamp}.sh", std::process::id());
    let script = temp_dir.join(&name);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&script)
        .with_context(|| {
            format!(
                "could not create temporary MSYS2 script {}",
                script.display()
            )
        })?;

    let execution = (|| -> Result<()> {
        file.write_all(&script_contents(command))
            .with_context(|| format!("could not write {}", script.display()))?;
        file.flush()
            .with_context(|| format!("could not flush {}", script.display()))?;
        drop(file);

        let relative_script = format!(".pixi/tmp/{name}");
        run_bash(
            env,
            &[
                OsStr::new("--noprofile"),
                OsStr::new("--norc"),
                OsStr::new(&relative_script),
            ],
            child_environment,
        )
    })();

    let cleanup = fs::remove_file(&script).with_context(|| {
        format!(
            "could not remove temporary MSYS2 script {}",
            script.display()
        )
    });
    match (execution, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(cleanup_error)) => Err(error.context(format!(
            "also failed to remove temporary script: {cleanup_error:#}"
        ))),
    }
}

fn script_contents(command: &str) -> Vec<u8> {
    let mut script = String::with_capacity(PREAMBLE.len() + command.len() + 1);
    script.push_str(PREAMBLE);
    script.push_str(command);
    if !command.ends_with('\n') {
        script.push('\n');
    }
    script.into_bytes()
}

fn run_bash(
    env: &PreparedEnvironment,
    arguments: &[&OsStr],
    child_environment: &[(&str, &OsStr)],
) -> Result<()> {
    let bash = env.conda.join("Library/usr/bin/bash.exe");
    ensure!(
        bash.is_file(),
        "Pixi-managed MSYS2 bash was not found at {}",
        bash.display()
    );

    let mut process = Command::new(&bash);
    process
        .args(arguments)
        .current_dir(&env.root)
        .envs(child_environment.iter().copied())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    println!("> {process:?}");
    io::stdout().flush().context("could not flush stdout")?;

    let status = process.status().with_context(|| {
        format!(
            "could not start Pixi-managed MSYS2 bash at {}",
            bash.display()
        )
    })?;
    if !status.success() {
        if let Some(code) = status.code() {
            bail!("MSYS2 command failed with exit code {code}");
        }
        bail!("MSYS2 command was terminated without an exit code");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{script_contents, PREAMBLE};

    #[test]
    fn generated_script_has_the_preamble_and_lf_line_endings() {
        let script = script_contents("make check-smoke");
        let expected = format!("{PREAMBLE}make check-smoke\n").into_bytes();

        assert_eq!(script, expected);
        assert!(!script.contains(&b'\r'));
    }
}
