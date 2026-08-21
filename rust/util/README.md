# Rust development utilities

This directory contains project-owned Rust tooling. It is intentionally separate from the upstream `util/` tree so upstream changes can be pulled without mixing them with the native Windows and Rust testsuite migration infrastructure.

- `test-plan/` defines the versioned declarative runtime format.
- `testsuite-manifest/` parses upstream Tcl test declarations and lowers them into typed manifests and Test Plans without executing Tcl.
- `xtask/` provides the cross-platform development task implementation exposed through the root `Justfile`.
- `z3-bridge/` exposes the Pixi-managed Z3 solver to the existing BSC code through a stable C ABI.

All crates are members of the root Cargo workspace. User-facing commands remain in the root `Justfile`; reusable orchestration belongs in `xtask`, parser/importer semantics belong in `testsuite-manifest`, and runtime operations belong in `rust/tests`.
