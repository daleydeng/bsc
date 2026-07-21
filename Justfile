# BSC native Windows build recipes. Pixi supplies the base toolchain; OSS CAD Suite supplies Icarus.

ps := "powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File util/windows/pixi.ps1"

# List available recipes.
default:
    @just --list --unsorted

# Configure an existing OSS CAD Suite installation for Icarus simulation.
configure-oss-cad-suite root:
    {{ps}} configure-oss-cad-suite "{{root}}"

# Install and select the project-local GHC and Cabal toolchain.
toolchain:
    {{ps}} toolchain

# Install BSC's Haskell package dependencies using the configured mirror.
haskell-deps: toolchain
    {{ps}} haskell-deps

# Prepare the complete native Windows build environment.
bootstrap: haskell-deps

# Report the exact tools and platform seen by the build.
doctor: bootstrap
    {{ps}} doctor

# Build and install BSC into ./inst.
build: haskell-deps
    {{ps}} build

# Build BSC and run the upstream smoke test.
smoke: build
    {{ps}} smoke

# Run the migrated scheduler SAT tests against Z3 with Rust.
# The Rust harness reports a clear error if `pixi run just build` has not run yet.
test-z3:
    {{ps}} test-z3

# Check that Rust case declarations still match their upstream .exp origins.
test-alignment:
    {{ps}} test-alignment

# Run the dynamically migrated upstream tests after checking alignment.
test-upstream:
    {{ps}} test-upstream

# Run Rust harness tests and all migrated contract tests.
test-rust:
    {{ps}} test-rust

# Default test entry point with content-addressed BSC caches and ccache.
test:
    {{ps}} test

# Run the complete suite without generation or compiler cache reads/writes.
test-cold:
    {{ps}} test-cold

# Show Bluesim C++ compiler-cache statistics.
ccache-stats:
    {{ps}} ccache-stats

# Remove all cached Bluesim C++ compilation results.
ccache-clear:
    {{ps}} ccache-clear

# Remove the upstream build and installation directories.
clean:
    {{ps}} clean

# Enter the Pixi-managed MSYS2 shell used by the build.
msys2-shell:
    {{ps}} shell
