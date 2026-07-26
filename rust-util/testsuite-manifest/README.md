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
- typed Bluesim workflow actions for `compile_object_pass`, `link_objects_pass`, `sim_output`, `copy`, and `move`;
- conservative Bluesim workflow composition by producer/consumer guard coverage, top-level executable, link segment, and stdout artifact flow; ambiguous actions remain explicit review items;
- source spans plus procedure-call expansion spans;
- every unsupported construct explicitly, including its expansion stack.

Procedure expansion counts contract instances rather than syntax occurrences. For example, a procedure body containing nine contracts and called five times lowers to 45 typed contracts. Alignment and inventory consume these expanded typed contracts directly.

Manifest schema v4 currently composes 139 Bluesim workflows representing 152 effective run-or-link contracts. Of the original 1,027 workflow actions, 595 ambiguous or side-artifact actions remain uncomposed in 85 scripts. Static lowering associates 98 of 101 `sim_output` actions with a link workflow; the remaining three lack a safely composable typed link/generation chain.

The Rust runner now has a dedicated `BluesimWorkflowScenario` model for multi-generation, link-only, and ordered multi-run workflows plus stdout artifact snapshots. Alignment parses static Tcl lists without evaluation and compares each registered generation, link, run, and transfer against this IR. Nineteen end-to-end Windows migrations now cover the core shapes: `b1489.exp` for a single run and text assertions, `b1243.exp` for link-only execution, and `traffic_light_controller_separate.exp` for two generations, ordered runs, artifact snapshots, and goldens, plus seven `Library_latency` origins and `bsc.lib/sram/sram.exp` for 24 single-generation/link/run/golden workflows, and `debugging.exp`/`b1439.exp`/`b1796.exp` for six build-only workflows, plus `eq3.exp` and `parse_strings.exp` as complete mixed origins containing two additional build-only workflows, and `rdy_en_pragmas.exp` as a complete 23-contract mixed origin with one additional build-only workflow, plus `bsc.bluesim/schedule/schedule.exp` for four build-only workflows and generated C++ scheduler assertions, and `bsc.scheduler/use_cond/use_cond.exp` for twenty Verilog golden compile contracts and three Bug1741 build-only workflows. Repeated runs reuse the persistent build cache, while official `dumpbo`/`dumpba` checks still validate restored intermediate artifacts.

The remaining migration sequence is:

1. Batch-migrate composed workflows that have no remaining side actions or unsupported constructs.
2. Extend typed composition for side artifacts such as VCD and link-log snapshots.
3. Complete the allowlisted static control/value forms needed by upstream scripts.
4. Continue migrating the remaining typed contract inventory into executable Rust scenarios.

Unknown commands, dynamic substitutions, unsupported control flow, and non-constant values must remain explicit unsupported constructs. They must never be evaluated to make conversion succeed.
