# BSC testsuite manifest frontend

This crate is the only place where the Rust testsuite migration understands upstream Tcl syntax. It never evaluates Tcl or invokes a Tcl interpreter.

## Target architecture

```text
testsuite/**/*.exp
    -> Tree-sitter Tcl CST
    -> allowlisted static lowerer
    -> versioned typed contract manifest
    -> per-origin BSC Test Plan JSON
    -> canonical Rust runner
```

The typed manifest is an importer IR rather than a runtime format. `rust/tests` executes versioned Test Plans and never parses Tcl. `cargo xtask` owns update/check commands and filesystem orchestration; this crate owns parsing, lowering, ordered operation recovery, and `.exp` to Test Plan conversion.

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
- compile, simulation, assertion, comparison, and external contract sets; compile and simulation contracts retain their complete static argument vectors;
- typed Bluesim workflow actions for `compile_object_pass`, `link_objects_pass`, `sim_output`, `copy`, `move`, `erase`, and `mkdir`;
- conservative Bluesim workflow composition by producer/consumer guard coverage, top-level executable, link segment, stdout artifact flow, statically parsed `-V` VCD producers, and the declared `<top>.bsc-ccomp-out` link-log producer; ambiguous actions remain explicit review items;
- source spans plus procedure-call expansion spans;
- every unsupported construct explicitly, including its expansion stack.

Procedure expansion counts contract instances rather than syntax occurrences. For example, a procedure body containing nine contracts and called five times lowers to 45 typed contracts. Alignment and inventory consume these expanded typed contracts directly.

Manifest schema v10 currently recovers 3,435 compile contracts, 2,086 simulation contracts, and 24 external contracts. It composes two ordered Bluesim sequences representing six contracts and 135 Bluesim workflows representing 148 effective run-or-link contracts, for a combined total of 154. The sequence lowerer preserves action, assertion, and barrier order. It recognizes the three `-parallel-sim-link` contracts in `parallel.exp` as one shared-workspace sequence with step counts 2/3/2 and assertion counts 3/5/4, and the producer-proven erase/re-link chain in `b1894.exp` as a second sequence with step counts 2/4/3 and assertion counts 1/2/1. The manifest currently retains 772 ambiguous, generic filesystem, or side-artifact actions as uncomposed typed review items and records 1,558 unsupported constructs. Static lowering associates 98 of 101 `sim_output` actions with a link workflow. Six ordinary `move` actions for declared `<top>.bsc-ccomp-out` logs remain attached as `link_transfers`; generic generated `.cxx` producers remain uncomposed unless an erase/re-link sequence proves their artifact flow.

The Rust runner now has a dedicated `BluesimWorkflowScenario` model for multi-generation, link-only, and ordered multi-run workflows plus link-log, stdout, and statically declared VCD artifact snapshots. Link-stage transfers run after a successful cold link and before the complete build workspace is stored; a cache hit restores that snapshot and does not replay the transfer. Alignment parses static Tcl lists without evaluation and compares each registered generation, link, link transfer, run, and run transfer against this IR. Twenty-five end-to-end Windows migrations now cover the core shapes: `b1489.exp` for a single run and text assertions, `b1243.exp` for link-only execution, and `traffic_light_controller_separate.exp` for two generations, ordered runs, artifact snapshots, and goldens, plus seven `Library_latency` origins and `bsc.lib/sram/sram.exp` for 24 single-generation/link/run/golden workflows, and `debugging.exp`/`b1439.exp`/`b1796.exp` for six build-only workflows, plus `eq3.exp` and `parse_strings.exp` as complete mixed origins containing two additional build-only workflows, and `rdy_en_pragmas.exp` as a complete 23-contract mixed origin with one additional build-only workflow, plus `bsc.bluesim/schedule/schedule.exp` for four build-only workflows and generated C++ scheduler assertions, and `bsc.scheduler/use_cond/use_cond.exp` for twenty Verilog golden compile contracts and three Bug1741 build-only workflows, plus four active build-only workflows under `bsc.interra/bluesim/interactive` whose commented interactive runs remain intentionally outside the contract inventory, and `array.exp`/`handshake_protocol_cl.exp` for three explicit/default `-V` VCD run-and-transfer contracts. Repeated runs reuse the persistent build cache, while official `dumpbo`/`dumpba` checks still validate restored intermediate artifacts.

The Rust runner executes both ordered sequences in shared workspaces with dependency-prefix filtering, strict transfers and erases, contract-local failure reporting, and persistent full-sequence caches. Contract-boundary snapshots retain only assertion actual files, so cache hits rerun assertions for artifacts later moved or erased without duplicating the whole workspace. All six sequence contracts pass on Windows in cold and warm runs. The special-character `parallel.exp` `-simdir` simulation remains byte-for-byte faithful on POSIX and is explicitly skipped on Windows because `:` cannot occur in a Windows directory component.

The importer emits one JSON Test Plan for every one of the 860 contract origins. Test Plan schema v4 currently contains 351 complete and 509 explicitly blocked plans, with 3,677 scenarios, 3,773 stages, 7,432 operations, and 4,840 diagnostics. The generic `bsc.compile` emitter supports frontend, Bluesim object, Verilog, and schedule modes across thirteen statically recovered helper shapes, including complete argv, dependency mode, expected success/failure, output artifacts, `.bo` validation, and immediately following assertions or comparisons. The 67 compile_object_fail and compile_object_fail_error calls are now typed bluesim_object failure contracts; a real SimpleDynamicBounds scenario passes on Windows, while its mixed origin remains blocked by unconverted simulation contracts. Compile producers and Bluesim build/run producers share one barrier-aware producer window; contracts, unsupported constructs, uncomposed actions, guard mismatches, and undeclared artifacts prevent speculative attachment. Executable compile and generation inputs receive a recursive same-fixture-directory dependency closure for BSV `import Package::*`, BH `import Package` / `import qualified Package`, and includes. Local package ambiguity and invalid includes remain fail-closed. Windows extensionless Bluesim launchers use a typed platform adapter rather than a plan-level shell escape hatch. `pixi run just plans-update`, `plans-check`, and `plans-audit` own deterministic generation and inventory gates. The canonical runner executes complete plans in parallel with Rayon (`--jobs N`), while scenario, stage, and operation order inside each plan remains strict. An intermediate 349-plan Windows run passed 1,458 stages and skipped one explicit non_windows stage; its only failures were the BNotShared.bsv timeout and the subsequently fixed b568 producer binding. After that fix, b568 and all 38 newly affected complete plans passed exact runs. The six origins completed by generic test_c_only_bsv lowering passed all nine stages in cold execution and again with nine generation-cache hits. The current 351-plan set has not yet been rerun as one full command; BNotShared.bsv remains the only known runtime blocker.

The remaining migration sequence is:

1. Model typed cross-backend simulation generation, link, run, and golden contracts without exposing shell semantics.
2. Model bug/xfail, Verilog link/simulation, Bluetcl, scheduler, and generated-fixture workflows.
3. Extend producer modeling only when a concrete upstream origin proves an additional artifact.

Unknown commands, dynamic substitutions, unsupported control flow, and non-constant values must remain explicit unsupported constructs. They must never be evaluated to make conversion succeed.
