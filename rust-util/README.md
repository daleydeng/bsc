# Rust development utilities

This directory contains project-owned Rust tooling. It is intentionally separate from the upstream `util/` tree so upstream changes can be pulled without mixing them with the native Windows and Rust testsuite migration infrastructure.

- `xtask/` provides the cross-platform development task implementation exposed through the root `Justfile`.
- `testsuite-manifest/` parses upstream Tcl test declarations and lowers them into the typed Rust contract manifest without executing Tcl.

Both crates are members of the root Cargo workspace. User-facing commands should remain in the root `Justfile`; reusable orchestration belongs in `xtask`, while testsuite parsing and contract semantics belong in `testsuite-manifest`.
