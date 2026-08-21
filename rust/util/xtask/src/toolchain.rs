use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{anyhow, bail, ensure, Context, Result};
use sha2::{Digest, Sha256};
use tiny_http::{Response, Server, StatusCode};

use crate::environment::PreparedEnvironment;

const GHC_VERSION: &str = "9.6.7";
const GHC_WINDOWS_BINDIST: &str =
    "https://downloads.haskell.org/~ghc/9.6.7/ghc-9.6.7-x86_64-unknown-mingw32.tar.xz";
const GHC_WINDOWS_BINDIST_FILENAME: &str = "ghc-9.6.7-x86_64-unknown-mingw32.tar.xz";
const GHC_WINDOWS_BINDIST_SHA256: &str =
    "cf0e736ce4c875de0296426ee575eca177acf6b9b5c1dd4881b9fc79681e1d5f";
const CABAL_VERSION: &str = "3.10.3.0";
const HACKAGE_MIRROR: &str = "https://mirrors.ustc.edu.cn/hackage/";
const HASKELL_PACKAGES: &[&str] = &[
    "old-time",
    "regex-compat",
    "split",
    "strict-concurrency",
    "syb",
];

pub fn initialize(environment: &PreparedEnvironment) -> Result<()> {
    fs::create_dir_all(cabal_dir(environment)).with_context(|| {
        format!(
            "could not create Cabal directory {}",
            cabal_dir(environment).display()
        )
    })?;

    if !ghcup_has(environment, "ghc", GHC_VERSION)? {
        let bindist = verified_download(
            environment,
            GHC_WINDOWS_BINDIST,
            &environment
                .root
                .join(".pixi/downloads")
                .join(GHC_WINDOWS_BINDIST_FILENAME),
            GHC_WINDOWS_BINDIST_SHA256,
        )?;
        install_ghc_bindist(environment, &bindist)?;
    } else if command_output(environment, "ghc.exe", ["--numeric-version"])? != GHC_VERSION {
        run_command(prepared_command(environment, "ghcup.exe").args(["set", "ghc", GHC_VERSION]))?;
    }

    if !ghcup_has(environment, "cabal", CABAL_VERSION)? {
        run_command(prepared_command(environment, "ghcup.exe").args([
            "--cache",
            "install",
            "cabal",
            CABAL_VERSION,
        ]))?;
    } else if command_output(environment, "cabal.exe", ["--numeric-version"])? != CABAL_VERSION {
        run_command(prepared_command(environment, "ghcup.exe").args([
            "set",
            "cabal",
            CABAL_VERSION,
        ]))?;
    }

    run_command(prepared_command(environment, "ghc.exe").arg("--version"))?;
    run_command(prepared_command(environment, "cabal.exe").arg("--numeric-version"))?;
    Ok(())
}

pub fn install_dependencies(environment: &PreparedEnvironment) -> Result<()> {
    set_cabal_mirror(environment)?;

    let mut missing = Vec::new();
    for &package in HASKELL_PACKAGES {
        let output = prepared_command(environment, "ghc-pkg.exe")
            .args(["list", "--simple-output", package])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
            .with_context(|| format!("could not query GHC package {package}"))?;
        if !output.status.success() || output.stdout.iter().all(u8::is_ascii_whitespace) {
            missing.push(package);
        }
    }

    if missing.is_empty() {
        println!("Haskell dependencies are already installed.");
        return Ok(());
    }

    println!("Installing Haskell packages: {}", missing.join(", "));
    run_command(prepared_command(environment, "cabal.exe").arg("update"))?;
    run_command(
        prepared_command(environment, "cabal.exe")
            .arg("v1-install")
            .args(HASKELL_PACKAGES),
    )?;
    Ok(())
}

fn prepared_command(environment: &PreparedEnvironment, program: impl AsRef<Path>) -> Command {
    let mut command = Command::new(program.as_ref());
    command
        .current_dir(&environment.root)
        .env("GHCUP_INSTALL_BASE_PREFIX", environment.root.join(".pixi"))
        .env("CABAL_DIR", cabal_dir(environment))
        .env("CABAL_CONFIG", cabal_config(environment));
    command
}

fn cabal_dir(environment: &PreparedEnvironment) -> PathBuf {
    environment.root.join(".pixi/cabal")
}

fn cabal_config(environment: &PreparedEnvironment) -> PathBuf {
    cabal_dir(environment).join("config")
}

fn run_command(command: &mut Command) -> Result<()> {
    let rendered = format!("{command:?}");
    println!("> {rendered}");
    let status = command
        .status()
        .with_context(|| format!("could not run {rendered}"))?;
    ensure_command_succeeded(status, &rendered, None)
}

fn command_output<const N: usize>(
    environment: &PreparedEnvironment,
    program: &str,
    arguments: [&str; N],
) -> Result<String> {
    let mut command = prepared_command(environment, program);
    command.args(arguments);
    let rendered = format!("{command:?}");
    let output = command
        .output()
        .with_context(|| format!("could not run {rendered}"))?;
    ensure_command_succeeded(output.status, &rendered, Some(&output.stderr))?;
    String::from_utf8(output.stdout)
        .with_context(|| format!("{rendered} produced non-UTF-8 output"))
        .map(|output| output.trim().to_owned())
}

fn ensure_command_succeeded(
    status: ExitStatus,
    rendered: &str,
    stderr: Option<&[u8]>,
) -> Result<()> {
    if status.success() {
        return Ok(());
    }

    let detail = stderr
        .map(String::from_utf8_lossy)
        .map(|stderr| stderr.trim().to_owned())
        .filter(|stderr| !stderr.is_empty())
        .map(|stderr| format!("\n{stderr}"))
        .unwrap_or_default();
    if let Some(code) = status.code() {
        bail!("command failed with exit code {code}: {rendered}{detail}");
    }
    bail!("command terminated without an exit code: {rendered}{detail}")
}

fn ghcup_has(environment: &PreparedEnvironment, tool: &str, version: &str) -> Result<bool> {
    let status = prepared_command(environment, "ghcup.exe")
        .args(["whereis", tool, version])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .with_context(|| format!("could not query ghcup for {tool} {version}"))?;
    Ok(status.success())
}

fn verified_download(
    environment: &PreparedEnvironment,
    url: &str,
    destination: &Path,
    expected_sha256: &str,
) -> Result<PathBuf> {
    if destination.is_file() && sha256_file(destination)? != expected_sha256 {
        fs::remove_file(destination).with_context(|| {
            format!(
                "could not remove invalid download {}",
                destination.display()
            )
        })?;
    }

    if !destination.is_file() {
        let parent = destination
            .parent()
            .context("download destination has no parent directory")?;
        fs::create_dir_all(parent)
            .with_context(|| format!("could not create download directory {}", parent.display()))?;

        let curl = environment.conda.join("Library/bin/curl.exe");
        ensure!(
            curl.is_file(),
            "Pixi-managed curl was not found at {}",
            curl.display()
        );
        let download = run_command(
            prepared_command(environment, &curl).args([
                "--fail",
                "--location",
                "--retry",
                "3",
                "--output",
                destination
                    .to_str()
                    .context("download path is not valid Unicode")?,
                url,
            ]),
        );
        if let Err(error) = download {
            let _ = fs::remove_file(destination);
            return Err(error);
        }

        let actual = sha256_file(destination)?;
        if actual != expected_sha256 {
            let _ = fs::remove_file(destination);
            bail!("SHA256 mismatch for {url}; expected {expected_sha256}, got {actual}");
        }
    }

    Ok(destination.to_owned())
}

fn sha256_file(path: &Path) -> Result<String> {
    let file = File::open(path).with_context(|| format!("could not open {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .with_context(|| format!("could not read {}", path.display()))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

#[cfg(test)]
fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn install_ghc_bindist(environment: &PreparedEnvironment, bindist: &Path) -> Result<()> {
    let server = BindistServer::start(bindist)?;
    let command_result = run_command(prepared_command(environment, "ghcup.exe").args([
        "--no-verify",
        "install",
        "ghc",
        "--url",
        server.url(),
        GHC_VERSION,
        "--set",
    ]));
    let shutdown_result = server.shutdown();
    command_result?;
    shutdown_result
}

struct BindistServer {
    url: String,
    server: Arc<Server>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<Result<()>>>,
}

impl BindistServer {
    fn start(bindist: &Path) -> Result<Self> {
        let filename = bindist
            .file_name()
            .and_then(|name| name.to_str())
            .context("GHC bindist filename is not valid Unicode")?;
        ensure!(
            filename
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_')),
            "GHC bindist filename is not URL-safe: {filename}"
        );

        let server = Arc::new(
            Server::http("127.0.0.1:0")
                .map_err(|error| anyhow!("could not start temporary bindist server: {error}"))?,
        );
        let address = server
            .server_addr()
            .to_ip()
            .context("temporary bindist server did not bind an IP socket")?;
        let request_path = format!("/{filename}");
        let url = format!("http://127.0.0.1:{}/{}", address.port(), filename);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_server = Arc::clone(&server);
        let worker_stop = Arc::clone(&stop);
        let bindist = bindist.to_owned();
        let thread = thread::Builder::new()
            .name("ghc-bindist-server".to_owned())
            .spawn(move || serve_bindist(worker_server, worker_stop, bindist, request_path))
            .context("could not start temporary bindist server thread")?;

        Ok(Self {
            url,
            server,
            stop,
            thread: Some(thread),
        })
    }

    fn url(&self) -> &str {
        &self.url
    }

    fn shutdown(mut self) -> Result<()> {
        self.stop_and_join()
    }

    fn stop_and_join(&mut self) -> Result<()> {
        self.stop.store(true, Ordering::Release);
        self.server.unblock();
        let Some(thread) = self.thread.take() else {
            return Ok(());
        };
        thread
            .join()
            .map_err(|_| anyhow!("temporary bindist server thread panicked"))?
    }
}

impl Drop for BindistServer {
    fn drop(&mut self) {
        let _ = self.stop_and_join();
    }
}

fn serve_bindist(
    server: Arc<Server>,
    stop: Arc<AtomicBool>,
    bindist: PathBuf,
    request_path: String,
) -> Result<()> {
    while !stop.load(Ordering::Acquire) {
        let request = match server.recv_timeout(Duration::from_millis(250)) {
            Ok(Some(request)) => request,
            Ok(None) => continue,
            Err(_) if stop.load(Ordering::Acquire) => break,
            Err(error) => return Err(error).context("temporary bindist server failed"),
        };

        if request.url() == request_path {
            let file = File::open(&bindist)
                .with_context(|| format!("could not open GHC bindist {}", bindist.display()))?;
            request
                .respond(Response::from_file(file))
                .context("could not serve GHC bindist")?;
        } else {
            request
                .respond(Response::empty(StatusCode(404)))
                .context("could not send bindist server 404 response")?;
        }
    }
    Ok(())
}

fn set_cabal_mirror(environment: &PreparedEnvironment) -> Result<()> {
    let directory = cabal_dir(environment);
    let config_path = cabal_config(environment);
    fs::create_dir_all(&directory)
        .with_context(|| format!("could not create Cabal directory {}", directory.display()))?;
    if !config_path.is_file() {
        run_command(prepared_command(environment, "cabal.exe").args(["user-config", "init"]))?;
    }

    let config = fs::read(&config_path)
        .with_context(|| format!("could not read Cabal config {}", config_path.display()))?;
    ensure!(
        config.is_ascii(),
        "Cabal config is not ASCII: {}",
        config_path.display()
    );
    let config = std::str::from_utf8(&config).expect("ASCII is valid UTF-8");
    let updated = cabal_config_with_mirror(config).with_context(|| {
        format!(
            "could not locate the hackage.haskell.org URL in {}",
            config_path.display()
        )
    })?;
    if updated != config {
        fs::write(&config_path, updated.as_bytes())
            .with_context(|| format!("could not update Cabal config {}", config_path.display()))?;
    }
    Ok(())
}

fn cabal_config_with_mirror(config: &str) -> Option<String> {
    let mut found = false;
    let mut updated = String::with_capacity(config.len());

    for line in config.split_inclusive('\n') {
        let Some((prefix, url, ending)) = cabal_url_line(line) else {
            updated.push_str(line);
            continue;
        };
        if matches!(
            url,
            "http://hackage.haskell.org"
                | "http://hackage.haskell.org/"
                | "https://hackage.haskell.org"
                | "https://hackage.haskell.org/"
        ) {
            found = true;
            updated.push_str(prefix);
            updated.push_str(HACKAGE_MIRROR);
            updated.push_str(ending);
        } else {
            found |= url == HACKAGE_MIRROR;
            updated.push_str(line);
        }
    }

    found.then_some(updated)
}

fn cabal_url_line(line: &str) -> Option<(&str, &str, &str)> {
    let (body, ending) = if let Some(body) = line.strip_suffix("\r\n") {
        (body, "\r\n")
    } else if let Some(body) = line.strip_suffix('\n') {
        (body, "\n")
    } else {
        (line, "")
    };
    let after_indent = body.trim_start_matches(|character| matches!(character, ' ' | '\t'));
    let after_label = after_indent.strip_prefix("url:")?;
    let url_part = after_label.trim_start_matches(|character| matches!(character, ' ' | '\t'));
    let prefix_length = body.len() - url_part.len();
    let url = url_part.trim_end_matches(|character| matches!(character, ' ' | '\t'));
    Some((&body[..prefix_length], url, ending))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_sha256() {
        assert_eq!(
            sha256_bytes(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn replaces_only_hackage_repository_url() {
        let config = "repository hackage.haskell.org\r\n  url: http://hackage.haskell.org/  \r\n  secure: True\r\n-- url: https://hackage.haskell.org/\r\nrepository private\r\n  url: https://example.com/hackage/\r\n";
        let expected = "repository hackage.haskell.org\r\n  url: https://mirrors.ustc.edu.cn/hackage/\r\n  secure: True\r\n-- url: https://hackage.haskell.org/\r\nrepository private\r\n  url: https://example.com/hackage/\r\n";

        assert_eq!(cabal_config_with_mirror(config).as_deref(), Some(expected));
    }

    #[test]
    fn accepts_an_already_configured_mirror() {
        let config =
            "repository hackage.haskell.org\n  url: https://mirrors.ustc.edu.cn/hackage/\n";
        assert_eq!(cabal_config_with_mirror(config).as_deref(), Some(config));
    }

    #[test]
    fn rejects_config_without_hackage_repository_url() {
        let config = "repository private\n  url: https://example.com/hackage/\n";
        assert!(cabal_config_with_mirror(config).is_none());
    }
}
