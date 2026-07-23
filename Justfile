# BSC native Windows build recipes. Pixi supplies the base toolchain; OSS CAD Suite supplies Icarus.

runner := "python util/windows/pixi.py"

# List available recipes.
default:
    @just --list --unsorted

# Configure an existing OSS CAD Suite installation for Icarus simulation.
configure-oss-cad-suite root:
    {{runner}} configure-oss-cad-suite "{{root}}"

# Install and select the project-local GHC and Cabal toolchain.
toolchain:
    {{runner}} toolchain

# Install BSC's Haskell package dependencies using the configured mirror.
haskell-deps: toolchain
    {{runner}} haskell-deps

# Prepare the complete native Windows build environment.
bootstrap: haskell-deps

# Report the exact tools and platform seen by the build.
doctor: bootstrap
    {{runner}} doctor

# Build and install BSC into ./inst.
build: haskell-deps
    {{runner}} build

# Build BSC and run the upstream smoke test.
smoke: build
    {{runner}} smoke

# Run the migrated scheduler SAT tests against Z3 with Rust.
# The Rust harness reports a clear error if `pixi run just build` has not run yet.
test-z3:
    {{runner}} test-z3

# Check that Rust case declarations still match their upstream .exp origins.
test-alignment:
    {{runner}} test-alignment

# Check that the generated complete remaining-tests inventory is current.
inventory-check:
    {{runner}} inventory-check

# Regenerate rust-tests/REMAINING.md from the alignment registry and testsuite.
inventory-update:
    {{runner}} inventory-update

# Run migrated upstream tests after checking alignment; optional arguments are forwarded to the Rust runner.
test-upstream *args:
    {{runner}} test-upstream {{args}}

# Run Rust harness tests and all migrated contract tests.
test-rust:
    {{runner}} test-rust

# Default test entry point with content-addressed BSC caches and ccache.
test:
    {{runner}} test

# Run the complete suite without generation or compiler cache reads/writes.
test-cold:
    {{runner}} test-cold

# Show Bluesim C++ compiler-cache statistics.
ccache-stats:
    {{runner}} ccache-stats

# Remove all cached Bluesim C++ compilation results.
ccache-clear:
    {{runner}} ccache-clear

# Remove the upstream build and installation directories.
clean:
    {{runner}} clean

# Enter the Pixi-managed MSYS2 shell used by the build.
msys2-shell:
    {{runner}} shell
