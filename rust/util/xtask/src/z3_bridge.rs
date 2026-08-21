use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
};

use anyhow::{bail, ensure, Context, Result};

use crate::environment::PreparedEnvironment;

const PACKAGE: &str = "bsc-z3-bridge";
const TARGET: &str = "x86_64-pc-windows-msvc";
const DLL_NAME: &str = "bsc_z3_bridge.dll";
const MSVC_IMPORT_LIBRARY: &str = "bsc_z3_bridge.dll.lib";
const GNU_IMPORT_LIBRARY: &str = "libbsc_z3_bridge.a";
const Z3_DLL_NAME: &str = "libz3.dll";

pub fn build(environment: &PreparedEnvironment) -> Result<()> {
    ensure!(
        cfg!(windows),
        "the native Z3 bridge build currently requires Windows"
    );

    let crate_dir = environment.root.join("rust/util/z3-bridge");
    let staging = staging_dir(environment);
    let include_dir = staging.join("include");
    let library_dir = staging.join("lib");
    let binary_dir = staging.join("bin");
    fs::create_dir_all(&include_dir)
        .with_context(|| format!("could not create {}", include_dir.display()))?;
    fs::create_dir_all(&library_dir)
        .with_context(|| format!("could not create {}", library_dir.display()))?;
    fs::create_dir_all(&binary_dir)
        .with_context(|| format!("could not create {}", binary_dir.display()))?;

    generate_header(&crate_dir, &include_dir.join("bsc_z3_bridge.h"))?;

    let cargo = env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
    let z3_header = environment.conda.join("Library/include/z3.h");
    let z3_library_dir = environment.conda.join("Library/lib");
    ensure!(
        z3_header.is_file(),
        "Pixi-managed Z3 header is missing: {}",
        z3_header.display()
    );
    ensure!(
        z3_library_dir.join("libz3.lib").is_file(),
        "Pixi-managed Z3 import library is missing: {}",
        z3_library_dir.join("libz3.lib").display()
    );

    run(
        Command::new(cargo)
            .current_dir(&environment.root)
            .env("Z3_SYS_Z3_HEADER", &z3_header)
            .env("Z3_LIBRARY_PATH_OVERRIDE", &z3_library_dir)
            .args([
                "build",
                "--locked",
                "--package",
                PACKAGE,
                "--release",
                "--target",
                TARGET,
                "--jobs",
                &environment.jobs.to_string(),
            ]),
        "Rust Z3 bridge build",
    )?;

    let release_dir = cargo_target_dir(environment).join(TARGET).join("release");
    copy_file(&release_dir.join(DLL_NAME), &binary_dir.join(DLL_NAME))?;
    copy_file(
        &environment.conda.join("Library/bin").join(Z3_DLL_NAME),
        &binary_dir.join(Z3_DLL_NAME),
    )?;

    let definition = staging.join("bsc_z3_bridge.def");
    fs::write(
        &definition,
        "LIBRARY bsc_z3_bridge.dll\nEXPORTS\n  bsc_z3_check_smtlib2\n  bsc_z3_version\n",
    )
    .with_context(|| format!("could not write {}", definition.display()))?;
    run(
        Command::new("dlltool.exe").args([
            "--input-def",
            path_text(&definition)?,
            "--dllname",
            DLL_NAME,
            "--output-lib",
            path_text(&library_dir.join(GNU_IMPORT_LIBRARY))?,
            "--machine",
            "i386:x86-64",
        ]),
        "MinGW Z3 bridge import-library generation",
    )?;

    let msvc_import_library = release_dir.join(MSVC_IMPORT_LIBRARY);
    if msvc_import_library.is_file() {
        copy_file(&msvc_import_library, &library_dir.join(MSVC_IMPORT_LIBRARY))?;
    }

    println!("Staged Rust Z3 bridge in {}", staging.display());
    Ok(())
}

pub fn install_runtime(environment: &PreparedEnvironment) -> Result<()> {
    let source = staging_dir(environment).join("bin");
    let destination = environment.root.join("inst/bin/core");
    fs::create_dir_all(&destination)
        .with_context(|| format!("could not create {}", destination.display()))?;
    for name in [DLL_NAME, Z3_DLL_NAME] {
        copy_file(&source.join(name), &destination.join(name))?;
    }
    Ok(())
}

pub fn clean(environment: &PreparedEnvironment) -> Result<()> {
    let staging = staging_dir(environment);
    match fs::remove_dir_all(&staging) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("could not remove {}", staging.display())),
    }
}

fn generate_header(crate_dir: &Path, destination: &Path) -> Result<()> {
    let config = cbindgen::Config::from_file(crate_dir.join("cbindgen.toml"))
        .map_err(|error| anyhow::anyhow!("could not read cbindgen configuration: {error}"))?;
    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .map_err(|error| anyhow::anyhow!("could not generate Z3 bridge header: {error}"))?
        .write_to_file(destination);
    ensure!(
        destination.is_file(),
        "cbindgen did not create {}",
        destination.display()
    );
    Ok(())
}

fn staging_dir(environment: &PreparedEnvironment) -> PathBuf {
    environment.root.join(".pixi/tmp/z3-bridge")
}

fn cargo_target_dir(environment: &PreparedEnvironment) -> PathBuf {
    env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| environment.root.join("target"))
}

fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    ensure!(
        source.is_file(),
        "required file is missing: {}",
        source.display()
    );
    fs::copy(source, destination).with_context(|| {
        format!(
            "could not copy {} to {}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(())
}

fn path_text(path: &Path) -> Result<&str> {
    path.to_str()
        .with_context(|| format!("path is not valid Unicode: {}", path.display()))
}

fn run(command: &mut Command, description: &str) -> Result<()> {
    let rendered = format!("{command:?}");
    println!("> {rendered}");
    let status = command
        .status()
        .with_context(|| format!("could not start {description}: {rendered}"))?;
    ensure_success(status, description)
}

fn ensure_success(status: ExitStatus, description: &str) -> Result<()> {
    if status.success() {
        return Ok(());
    }
    if let Some(code) = status.code() {
        bail!("{description} failed with exit code {code}");
    }
    bail!("{description} was terminated without an exit code")
}
