# BSC native Windows build recipes. Pixi supplies the base toolchain; OSS CAD Suite supplies Icarus.

pixi_cargo := env("CONDA_PREFIX") + "/Library/bin/cargo.exe"
pixi_rtk := env("CONDA_PREFIX") + "/Library/bin/rtk.exe"

# List available recipes.
default:
    @"{{pixi_rtk}}" proxy just --list --unsorted

# Configure an existing OSS CAD Suite installation for Icarus simulation.
configure-oss-cad-suite root:
    "{{pixi_rtk}}" summary "{{pixi_cargo}}" xtask configure-oss-cad-suite "{{root}}"

# Install and select the project-local GHC and Cabal toolchain.
toolchain:
    "{{pixi_rtk}}" err "{{pixi_cargo}}" xtask toolchain

# Install BSC's Haskell package dependencies using the configured mirror.
haskell-deps: toolchain
    "{{pixi_rtk}}" err "{{pixi_cargo}}" xtask haskell-deps

# Prepare the complete native Windows build environment.
bootstrap: haskell-deps

# Report the exact tools and platform seen by the build.
doctor: bootstrap
    "{{pixi_rtk}}" summary "{{pixi_cargo}}" xtask doctor

# Build and install BSC into ./inst.
build: haskell-deps
    "{{pixi_rtk}}" err "{{pixi_cargo}}" xtask build

# Build BSC and run the upstream smoke test.
smoke: build
    "{{pixi_rtk}}" test "{{pixi_cargo}}" xtask smoke

# Run the migrated scheduler SAT tests against Z3 with Rust.
# The Rust harness reports a clear error if `pixi run just build` has not run yet.
test-z3:
    "{{pixi_rtk}}" test "{{pixi_cargo}}" xtask test-z3

# Check that Rust case declarations still match their upstream .exp origins.
test-alignment:
    "{{pixi_rtk}}" test "{{pixi_cargo}}" xtask test-alignment

# Check that the generated complete remaining-tests inventory is current.
inventory-check:
    "{{pixi_rtk}}" test "{{pixi_cargo}}" xtask inventory-check

# Regenerate rust-tests/REMAINING.md from the alignment registry and testsuite.
inventory-update:
    "{{pixi_rtk}}" err "{{pixi_cargo}}" xtask inventory-update

# Run migrated upstream tests with live, compact progress; optional arguments are forwarded to the Rust runner.
test-upstream *args:
    "{{pixi_rtk}}" proxy "{{pixi_cargo}}" xtask test-upstream {{args}}

# Run Rust harness tests and all migrated contract tests with live progress.
test-rust:
    "{{pixi_rtk}}" proxy "{{pixi_cargo}}" xtask test-rust

# Default test entry point with live progress, content-addressed BSC caches, and ccache.
test:
    "{{pixi_rtk}}" proxy "{{pixi_cargo}}" xtask test

# Run the complete suite with live progress and without generation or compiler caches.
test-cold:
    "{{pixi_rtk}}" proxy "{{pixi_cargo}}" xtask test-cold

# Remove disposable Rust test workspaces and diagnostics left by interrupted or failed runs.
test-prune:
    "{{pixi_rtk}}" proxy "{{pixi_cargo}}" xtask test-prune

# Show Bluesim C++ compiler-cache statistics.
ccache-stats:
    "{{pixi_rtk}}" summary "{{pixi_cargo}}" xtask ccache-stats

# Remove all cached Bluesim C++ compilation results.
ccache-clear:
    "{{pixi_rtk}}" err "{{pixi_cargo}}" xtask ccache-clear

# Remove the upstream build and installation directories.
clean:
    "{{pixi_rtk}}" err "{{pixi_cargo}}" xtask clean

# Enter the Pixi-managed MSYS2 shell used by the build.
msys2-shell:
    "{{pixi_rtk}}" proxy "{{pixi_cargo}}" xtask shell
