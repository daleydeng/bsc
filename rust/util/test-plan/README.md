# BSC Test Plan

`bsc-test-plan` defines the versioned, declarative format executed by the Rust
test runner. It deliberately contains no Tcl parser and no process execution.

The canonical representation is JSON. `schema.json` is generated from the Rust
model and committed under `rust/tests/plans/` together with one plan per
upstream test origin.

A plan may be `blocked` while the importer still sees unsupported legacy
semantics. Blocked plans remain useful for inventory and review, but the runner
must reject them. A `complete` plan contains only the finite operation vocabulary
defined by this crate; arbitrary shell commands and `eval` are not representable.
