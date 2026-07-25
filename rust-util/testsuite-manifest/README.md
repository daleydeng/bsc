# BSC testsuite manifest frontend

This crate is the only place where the Rust testsuite migration understands upstream Tcl syntax. It never evaluates Tcl or invokes a Tcl interpreter.

## Target architecture

```text
testsuite/**/*.exp
    -> Tree-sitter Tcl CST
    -> allowlisted static lowerer
    -> versioned typed contract manifest
    -> alignment / inventory / migrated Rust tests
```

`rust-tests` and the test runner must consume the typed manifest rather than Tcl syntax. `cargo xtask` owns update/check commands and filesystem orchestration; this crate owns parsing, lowering, and the manifest model.

## Parser dependency

The frontend pins [`tree-sitter-grammars/tree-sitter-tcl`](https://github.com/tree-sitter-grammars/tree-sitter-tcl) to an exact Git revision because the grammar does not currently publish a `tree-sitter-tcl` crate on crates.io.

Tree-sitter supplies syntax and source ranges only. BSC helper semantics, constant `set` values, capability gates, golden comparisons, and unsupported dynamic constructs remain the responsibility of a non-executing, allowlisted lowerer.

## Syntax frontend

Run:

```text
pixi run just contracts-parse-check
pixi run just contracts-cst testsuite/path/to/script.exp
```

The initial scan covers the same 860 contract scripts as alignment. It intentionally excludes the three DejaGnu infrastructure files:

- `testsuite/config/unix.exp`
- `testsuite/lib/bsc.exp`
- `testsuite/site.exp`

At the pinned revision, the unadapted grammar parses 807 of 860 contract scripts without syntax errors. The failures are grammar limitations rather than invalid Tcl: generic custom-helper arguments such as Verilog snippets and regular expressions in `{...}` are parsed as nested Tcl scripts. The crates.io `bca-tree-sitter-tcl` fork uses the same brace-word grammar, while `oak-tcl` currently requires nightly Rust and has a substantially less complete parser.

The frontend therefore parses an equal-length normalized view of each script:

- known assertion-helper brace arguments are allowlisted as opaque data;
- generic command brace arguments discovered from the CST are masked iteratively;
- balanced-brace recovery is limited to Tree-sitter error nodes and allowlisted helpers;
- Tcl line continuations, array-variable words, namespace-variable prefixes, and optional `then` tokens receive byte-length-preserving normalization rewrites;
- line and column diagnostics are calculated from the original source rather than the normalized view.

No rewrite changes a byte offset. The lowerer always slices values from the original `.exp` source by Tree-sitter byte range. Control-structure bodies remain strict and are never treated as opaque data.

The current full scan is structurally clean for all 860 contract scripts. At the current upstream revision it masks 2,388 opaque arguments and applies 644 normalization rewrites, with no residual syntax issues.

## Typed lowering

The current lowerer models:

- scalar and static-list `set` values;
- variable, quoted-word, concatenated-word, and `[list ...]` substitution when every component is static;
- capability and unresolved guards, including complementary `if/else` branches;
- non-recursive local procedure calls with static arguments;
- compile, simulation, assertion, comparison, and external contract sets;
- source spans plus procedure-call expansion spans;
- every unsupported construct explicitly, including its expansion stack.

Procedure expansion counts contract instances rather than syntax occurrences. For example, a procedure body containing nine contracts and called five times lowers to 45 typed contracts. Alignment and inventory consume these expanded typed contracts directly.

The remaining migration sequence is:

1. Complete the allowlisted static control/value forms needed by upstream scripts.
2. Continue migrating the remaining typed contract inventory into executable Rust scenarios.

Unknown commands, dynamic substitutions, unsupported control flow, and non-constant values must remain explicit unsupported constructs. They must never be evaluated to make conversion succeed.
