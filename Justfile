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

# Parse every upstream .exp file with Tree-sitter Tcl without executing Tcl.
contracts-parse-check:
    "{{pixi_rtk}}" test "{{pixi_cargo}}" xtask contracts-parse-check

# Lower every upstream .exp file into the typed contract IR.
contracts-ir-check:
    "{{pixi_rtk}}" test "{{pixi_cargo}}" xtask contracts-ir-check

# Verify that the committed typed contract manifest matches upstream .exp files.
contracts-check:
    "{{pixi_rtk}}" test "{{pixi_cargo}}" xtask contracts-check

# Regenerate the committed typed contract manifest.
contracts-update:
    "{{pixi_rtk}}" err "{{pixi_cargo}}" xtask contracts-update

# Verify all generated per-origin Test Plans, their index, and JSON Schema.
plans-check:
    "{{pixi_rtk}}" test "{{pixi_cargo}}" xtask plans-check

# Regenerate all per-origin Test Plans from upstream .exp files.
plans-update:
    "{{pixi_rtk}}" err "{{pixi_cargo}}" xtask plans-update

# Audit Test Plan coverage and non-.exp executable-input inventory.
plans-audit:
    "{{pixi_rtk}}" test "{{pixi_cargo}}" xtask plans-audit

# Execute complete Test Plans with the canonical Rust runner; optional arguments select plan IDs.
test-plans *args:
    "{{pixi_rtk}}" proxy "{{pixi_cargo}}" xtask test-plans {{args}}

# Print the Tree-sitter concrete syntax tree for one upstream .exp file.
contracts-cst script:
    "{{pixi_rtk}}" proxy "{{pixi_cargo}}" xtask contracts-cst "{{script}}"



# Check that the generated complete remaining-tests inventory is current.
inventory-check:
    "{{pixi_rtk}}" test "{{pixi_cargo}}" xtask inventory-check

# Regenerate rust/tests/REMAINING.md from Test Plan status and the typed manifest.
inventory-update:
    "{{pixi_rtk}}" err "{{pixi_cargo}}" xtask inventory-update



# Run Rust unit/SAT tests and all complete Test Plans with live progress.
test-rust:
    "{{pixi_rtk}}" proxy "{{pixi_cargo}}" xtask test-rust

# Default test entry point with live progress, content-addressed BSC caches, and sccache.
test:
    "{{pixi_rtk}}" proxy "{{pixi_cargo}}" xtask test

# Run the complete suite with live progress and without generation or compiler caches.
test-cold:
    "{{pixi_rtk}}" proxy "{{pixi_cargo}}" xtask test-cold

# Remove disposable Rust test workspaces and diagnostics left by interrupted or failed runs.
test-prune:
    "{{pixi_rtk}}" proxy "{{pixi_cargo}}" xtask test-prune

# Show shared Rust and Bluesim C++ compiler-cache statistics.
sccache-stats:
    "{{pixi_rtk}}" summary "{{pixi_cargo}}" xtask sccache-stats

# Remove all shared Rust and Bluesim C++ compilation results.
sccache-clear:
    "{{pixi_rtk}}" err "{{pixi_cargo}}" xtask sccache-clear

# Remove the upstream build and installation directories.
clean:
    "{{pixi_rtk}}" err "{{pixi_cargo}}" xtask clean

# Enter the Pixi-managed MSYS2 shell used by the build.
msys2-shell:
    "{{pixi_rtk}}" proxy "{{pixi_cargo}}" xtask shell
