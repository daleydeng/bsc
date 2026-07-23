from __future__ import annotations

import functools
import hashlib
import http.server
import os
import re
import shutil
import subprocess
import sys
import threading
import urllib.parse
import uuid
from pathlib import Path
from typing import NoReturn, Sequence

GHC_VERSION = "9.6.7"
GHC_WINDOWS_BINDIST = (
    "https://downloads.haskell.org/~ghc/9.6.7/"
    "ghc-9.6.7-x86_64-unknown-mingw32.tar.xz"
)
GHC_WINDOWS_BINDIST_SHA256 = (
    "cf0e736ce4c875de0296426ee575eca177acf6b9b5c1dd4881b9fc79681e1d5f"
)
CABAL_VERSION = "3.10.3.0"
HACKAGE_MIRROR = "https://mirrors.ustc.edu.cn/hackage/"
HASKELL_PACKAGES = ("old-time", "regex-compat", "split", "strict-concurrency", "syb")
ICARUS_ACTIONS = {"doctor", "smoke"}


def fail(message: str) -> NoReturn:
    raise RuntimeError(message)


def run(command: Sequence[str], *, cwd: Path | None = None) -> None:
    print(f"> {subprocess.list2cmdline(list(command))}", flush=True)
    result = subprocess.run(command, cwd=cwd, check=False)
    if result.returncode != 0:
        fail(
            f"Command failed with exit code {result.returncode}: "
            f"{subprocess.list2cmdline(list(command))}"
        )


def output(command: Sequence[str]) -> str:
    result = subprocess.run(
        command,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )
    if result.returncode != 0:
        fail(
            f"Command failed with exit code {result.returncode}: "
            f"{subprocess.list2cmdline(list(command))}\n{result.stderr.strip()}"
        )
    return result.stdout.strip()


def required_program(name: str) -> str:
    program = shutil.which(name)
    if program is None:
        fail(f"Required Pixi-managed program was not found: {name}")
    return program


def configured_jobs() -> int:
    raw = os.environ.get("BSC_JOBS", str(min(os.cpu_count() or 1, 16)))
    try:
        jobs = int(raw)
    except ValueError:
        fail("BSC_JOBS must be a positive integer.")
    if jobs < 1:
        fail("BSC_JOBS must be a positive integer.")
    return jobs


def windows_path(candidate: str) -> Path:
    candidate = candidate.strip().strip('"')
    match = re.fullmatch(r"/([A-Za-z])(?:/(.*))?", candidate)
    if match:
        suffix = (match.group(2) or "").replace("/", "\\")
        candidate = f"{match.group(1)}:\\{suffix}"
    return Path(candidate).resolve()


def valid_oss_root(candidate: str | None) -> Path | None:
    if not candidate or not candidate.strip():
        return None
    root = windows_path(candidate)
    required = (
        root / "environment.ps1",
        root / "bin" / "iverilog.exe",
        root / "bin" / "vvp.exe",
        root / "lib" / "ivl",
    )
    return root if all(path.exists() for path in required) else None


def resolve_oss_root(config: Path, *, required: bool) -> Path | None:
    candidates = [os.environ.get("OSS_CAD_SUITE_ROOT"), os.environ.get("YOSYSHQ_ROOT")]
    if config.is_file():
        candidates.append(config.read_text(encoding="utf-8").strip())
    iverilog = shutil.which("iverilog.exe")
    if iverilog:
        candidates.append(str(Path(iverilog).resolve().parent.parent))
    for candidate in candidates:
        root = valid_oss_root(candidate)
        if root is not None:
            return root
    if required:
        fail(
            "OSS CAD Suite with iverilog/vvp was not configured. Run "
            "'pixi run just configure-oss-cad-suite <path>' or set "
            "OSS_CAD_SUITE_ROOT."
        )
    return None


def save_oss_root(config: Path, candidate: str) -> None:
    root = valid_oss_root(candidate)
    if root is None:
        fail(f"Not a valid OSS CAD Suite root: {candidate}")
    config.parent.mkdir(parents=True, exist_ok=True)
    config.write_text(f"{root}\n", encoding="utf-8", newline="\n")
    print(f"Configured OSS CAD Suite: {root}")


def prepend_path(*paths: Path) -> None:
    current = os.environ.get("PATH", "")
    os.environ["PATH"] = os.pathsep.join([*(str(path) for path in paths), current])


def prepare_environment(root: Path, conda: Path, oss_root: Path | None, jobs: int) -> None:
    os.environ["BSC_BUILD_JOBS"] = str(jobs)
    os.environ["GHCUP_INSTALL_BASE_PREFIX"] = str(root / ".pixi")
    os.environ["CABAL_DIR"] = str(root / ".pixi" / "cabal")
    os.environ["CABAL_CONFIG"] = str(root / ".pixi" / "cabal" / "config")
    os.environ["MSYSTEM"] = "MINGW64"
    os.environ["CHERE_INVOKING"] = "1"

    ghcup = required_program("ghcup.exe")
    ghcup_bin = Path(output([ghcup, "whereis", "bindir"]))
    os.environ["BSC_GHCUP_BIN"] = str(ghcup_bin)

    mingw_bin = conda / "Library" / "mingw-w64" / "bin"
    msys_bin = conda / "Library" / "usr" / "bin"
    conda_bin = conda / "Library" / "bin"
    prepend_path(ghcup_bin, mingw_bin, msys_bin, conda_bin)

    if oss_root is not None:
        preferred_bin = root / ".pixi" / "tools" / "pixi-preferred-bin"
        preferred_bin.mkdir(parents=True, exist_ok=True)
        for name in ("z3.exe", "MSVCP140.dll", "VCRUNTIME140.dll", "VCRUNTIME140_1.dll"):
            source = conda_bin / name
            if not source.is_file():
                fail(f"Pixi-managed Z3 runtime file was not found: {source}")
            destination = preferred_bin / name
            if destination.is_file():
                try:
                    if os.path.samefile(source, destination):
                        continue
                except OSError:
                    pass
                if destination.stat().st_size == source.stat().st_size and sha256(
                    destination
                ) == sha256(source):
                    continue
            destination.unlink(missing_ok=True)
            try:
                os.link(source, destination)
            except OSError:
                shutil.copy2(source, destination)
        os.environ["OSS_CAD_SUITE_ROOT"] = str(oss_root)
        os.environ["BSC_OSS_CAD_SUITE_ROOT"] = str(oss_root)
        os.environ["BSC_PIXI_PREFERRED_BIN"] = str(preferred_bin)
        os.environ["YOSYSHQ_ROOT"] = f"{oss_root}{os.sep}"
        prepend_path(preferred_bin, oss_root / "bin", oss_root / "lib")

    required_program("ccache.exe")
    os.environ.setdefault("CCACHE_DIR", str(root / ".pixi" / "cache" / "ccache"))
    os.environ.setdefault("CCACHE_BASEDIR", str(root))
    os.environ.setdefault("CCACHE_MAXSIZE", "10G")
    os.environ.setdefault("CXX", "ccache c++")

    os.environ["PKG_CONFIG_PATH"] = os.pathsep.join(
        (
            str(conda / "Library" / "mingw-w64" / "lib" / "pkgconfig"),
            str(conda / "Library" / "mingw-w64" / "share" / "pkgconfig"),
        )
    )
    ca_bundle = conda / "Library" / "ssl" / "cacert.pem"
    os.environ["SSL_CERT_FILE"] = str(ca_bundle)
    os.environ["GIT_SSL_CAINFO"] = str(ca_bundle)
    os.environ["CURL_CA_BUNDLE"] = str(ca_bundle)
    os.environ["CARGO_TARGET_DIR"] = str(root / ".pixi" / "tmp" / "cargo-target")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def verified_download(url: str, destination: Path, expected_sha256: str, conda: Path) -> Path:
    if destination.is_file() and sha256(destination) != expected_sha256:
        destination.unlink()
    if not destination.is_file():
        destination.parent.mkdir(parents=True, exist_ok=True)
        curl = conda / "Library" / "bin" / "curl.exe"
        if not curl.is_file():
            fail(f"Pixi-managed curl was not found at {curl}.")
        run([str(curl), "--fail", "--location", "--retry", "3", "--output", str(destination), url])
        actual = sha256(destination)
        if actual != expected_sha256:
            destination.unlink(missing_ok=True)
            fail(f"SHA256 mismatch for {url}. Expected {expected_sha256}, got {actual}.")
    return destination


def ghcup_has(tool: str, version: str) -> bool:
    return (
        subprocess.run(
            ["ghcup.exe", "whereis", tool, version],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        ).returncode
        == 0
    )


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, format: str, *args: object) -> None:
        pass


def install_ghc_bindist(bindist: Path) -> None:
    handler = functools.partial(QuietHandler, directory=str(bindist.parent))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        filename = urllib.parse.quote(bindist.name)
        url = f"http://127.0.0.1:{server.server_port}/{filename}"
        run(["ghcup.exe", "--no-verify", "install", "ghc", "--url", url, GHC_VERSION, "--set"])
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


def initialize_toolchain(root: Path, conda: Path) -> None:
    Path(os.environ["CABAL_DIR"]).mkdir(parents=True, exist_ok=True)
    if not ghcup_has("ghc", GHC_VERSION):
        bindist = verified_download(
            GHC_WINDOWS_BINDIST,
            root / ".pixi" / "downloads" / "ghc-9.6.7-x86_64-unknown-mingw32.tar.xz",
            GHC_WINDOWS_BINDIST_SHA256,
            conda,
        )
        install_ghc_bindist(bindist)
    elif output(["ghc.exe", "--numeric-version"]) != GHC_VERSION:
        run(["ghcup.exe", "set", "ghc", GHC_VERSION])

    if not ghcup_has("cabal", CABAL_VERSION):
        run(["ghcup.exe", "--cache", "install", "cabal", CABAL_VERSION])
    elif output(["cabal.exe", "--numeric-version"]) != CABAL_VERSION:
        run(["ghcup.exe", "set", "cabal", CABAL_VERSION])

    run(["ghc.exe", "--version"])
    run(["cabal.exe", "--numeric-version"])


def set_cabal_mirror() -> None:
    cabal_dir = Path(os.environ["CABAL_DIR"])
    config_path = Path(os.environ["CABAL_CONFIG"])
    cabal_dir.mkdir(parents=True, exist_ok=True)
    if not config_path.is_file():
        run(["cabal.exe", "user-config", "init"])
    config = config_path.read_text(encoding="ascii")
    updated, count = re.subn(
        r"(?m)^(\s*url:\s*)https?://hackage\.haskell\.org/?\s*$",
        lambda match: f"{match.group(1)}{HACKAGE_MIRROR}",
        config,
    )
    if count == 0 and HACKAGE_MIRROR not in config:
        fail(f"Could not locate the hackage.haskell.org URL in {config_path}.")
    if updated != config:
        config_path.write_text(updated, encoding="ascii")


def install_haskell_dependencies() -> None:
    set_cabal_mirror()
    missing = []
    for package in HASKELL_PACKAGES:
        result = subprocess.run(
            ["ghc-pkg.exe", "list", "--simple-output", package],
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
        )
        if result.returncode != 0 or not result.stdout.strip():
            missing.append(package)
    if not missing:
        print("Haskell dependencies are already installed.")
        return
    print(f"Installing Haskell packages: {', '.join(missing)}")
    run(["cabal.exe", "update"])
    run(["cabal.exe", "v1-install", *HASKELL_PACKAGES])


def invoke_msys(root: Path, conda: Path, command: str | None = None) -> None:
    bash = conda / "Library" / "usr" / "bin" / "bash.exe"
    if not bash.is_file():
        fail(f"Pixi-managed MSYS2 bash was not found at {bash}.")
    if command is None:
        run([str(bash), "--noprofile", "--norc", "-i"], cwd=root)
        return

    temp_dir = root / ".pixi" / "tmp"
    temp_dir.mkdir(parents=True, exist_ok=True)
    script = temp_dir / f"pixi-task-{uuid.uuid4().hex}.sh"
    preamble = """export PATH="$(cygpath -u "$CONDA_PREFIX")/Library/bin:$PATH"
if [ -n "${BSC_OSS_CAD_SUITE_ROOT:-}" ]; then
    preferred_bin="$(cygpath -u "$BSC_PIXI_PREFERRED_BIN")"
    oss_cad_root="$(cygpath -u "$BSC_OSS_CAD_SUITE_ROOT")"
    export PATH="$preferred_bin:$oss_cad_root/bin:$oss_cad_root/lib:$PATH"
fi
export SSL_CERT_FILE="$(cygpath -u "$CONDA_PREFIX")/Library/ssl/cacert.pem"
export GIT_SSL_CAINFO="$SSL_CERT_FILE"
export CURL_CA_BUNDLE="$SSL_CERT_FILE"
"""
    script.write_text(f"{preamble}{command}\n", encoding="utf-8", newline="\n")
    try:
        run([str(bash), "--noprofile", "--norc", f".pixi/tmp/{script.name}"], cwd=root)
    finally:
        script.unlink(missing_ok=True)


def doctor_command() -> str:
    return """set -u
failed=0
for tool in bash make git diff gcc g++ ccache pkg-config tclsh iverilog vvp ghc ghc-pkg cabal rustc cargo z3; do
    if command -v "$tool" >/dev/null 2>&1; then
printf '%-12s %s\\n' "$tool" "$(command -v "$tool" | tr -d '\\r')"
    else
        printf '%-12s MISSING\\n' "$tool"
        failed=1
    fi
done
printf '%-12s %s\\n' OSTYPE "$(./platform.sh ostype | tr -d '\\r')"
printf '%-12s %s\\n' MACHTYPE "$(./platform.sh machtype | tr -d '\\r')"
printf '%-12s %s\\n' BUILD_JOBS "$BSC_BUILD_JOBS"
printf '%-12s %s\\n' OSS_CAD "$BSC_OSS_CAD_SUITE_ROOT"
iverilog -V 2>&1 | sed -n '1p' || true
ghc --version 2>/dev/null || true
cabal --numeric-version 2>/dev/null || true
rustc --version 2>/dev/null || true
cargo --version 2>/dev/null || true
z3 -version 2>/dev/null || true
exit "$failed"
"""


def dispatch(action: str, root: Path, conda: Path, jobs: int) -> None:
    if action == "toolchain":
        initialize_toolchain(root, conda)
    elif action == "haskell-deps":
        install_haskell_dependencies()
    elif action == "doctor":
        invoke_msys(root, conda, doctor_command())
    elif action == "build":
        ghc_temp = root / ".pixi" / "tmp" / "ghc"
        ghc_temp.mkdir(parents=True, exist_ok=True)
        os.environ["TEMP"] = str(ghc_temp)
        os.environ["TMP"] = str(ghc_temp)
        print(f"Building with {jobs} parallel jobs (set BSC_JOBS to override).")
        invoke_msys(root, conda, f"make -j{jobs} GHCJOBS={jobs} install-src")
    elif action == "smoke":
        invoke_msys(root, conda, "make check-smoke")
    elif action == "clean":
        invoke_msys(root, conda, "make full_clean")
    elif action == "shell":
        invoke_msys(root, conda)
    else:
        fail(f"Unknown action: {action}")


def main() -> None:
    if not os.environ.get("PIXI_PROJECT_ROOT") or not os.environ.get("CONDA_PREFIX"):
        fail("This script must run in the Pixi environment. Use 'pixi run just <recipe>'.")
    if len(sys.argv) < 2:
        fail("Missing action. Run 'pixi run just --list'.")

    root = Path(os.environ["PIXI_PROJECT_ROOT"]).resolve()
    conda = Path(os.environ["CONDA_PREFIX"]).resolve()
    config = root / ".pixi" / "oss-cad-suite-root.txt"
    action = sys.argv[1]
    if action == "configure-oss-cad-suite":
        if len(sys.argv) != 3:
            fail("Usage: pixi run just configure-oss-cad-suite <path>")
        save_oss_root(config, sys.argv[2])
        return
    if len(sys.argv) != 2:
        fail(f"Unexpected arguments for action: {action}")

    jobs = configured_jobs()
    oss_root = resolve_oss_root(config, required=action in ICARUS_ACTIONS)
    prepare_environment(root, conda, oss_root, jobs)
    dispatch(action, root, conda, jobs)


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1) from error
