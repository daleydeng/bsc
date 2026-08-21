# BSC Rust 测试层

`rust/tests` 是 upstream testsuite 迁移后的 canonical Rust 执行层。测试导入与执行严格分离：

```text
testsuite/**/*.exp
→ Tree-sitter Tcl CST
→ allowlisted static lowerer
→ typed manifest
→ versioned Test Plan JSON
→ canonical Rust runner
```

运行期只读取 `rust/tests/plans/*.test.json` 和已登记 fixture，不解释 Tcl，也不执行任意 shell/eval operation。`blocked` 计划必须 fail-closed。

## 运行

推荐从 Pixi/Just 入口执行，并在本机使用单线程以避免重型 BSC 测试占用过多资源：

```text
BSC_JOBS=1 pixi run just test
BSC_JOBS=1 pixi run just test-plans
BSC_JOBS=1 pixi run just test-plans bsc.bluesim/interactive/interactive --exact
BSC_JOBS=1 pixi run just test-plans --start-at bsc.lib/FShow/FShow
BSC_JOBS=1 pixi run just test-plans --list
BSC_JOBS=1 pixi run just test-z3
```

`test` 依次执行 contract、plan、audit、inventory 门禁，Rust helper/scheduler SAT tests，以及全部 complete Test Plans。大量输出应重定向到 `.pixi/tmp/benchmarks/`。Rust runner 使用 `std::process::Command` 启动 `bsc.exe`、Bluesim launcher、`iverilog`/`vvp` 等独立子进程，合并捕获 stdout/stderr，并由 scenario timeout 管理进程生命周期。

直接使用 Cargo 时，crate 位于 `rust/tests/Cargo.toml`，但调用者需自行保证 BSC、Z3 和 OSS CAD Suite 工具可发现。

## Source of truth

- `testsuite/**/*.exp`：upstream 测试契约来源，仅在 import/sync 阶段读取。
- `contracts/upstream-contracts.json`：静态 lowerer 产生的 typed importer IR 快照。
- `plans/index.json` 与 `plans/**/*.test.json`：唯一 runtime source of truth。
- `REMAINING.md`：由 plan status、typed manifest 和 blocker registry 自动生成的剩余工作清单。
- `ARCHITECTURE.md`：当前导入、artifact provenance、运行、缓存和 workflow composition 架构；也明确记录尚未完成的统一 versioned action graph 重构。
- `TEST_PLAN.md`：Test Plan 模型与迁移设计的历史/规范背景；动态统计与实际架构以 `ARCHITECTURE.md`、生成 plans 和 `REMAINING.md` 为准。

不再维护手写 `cases_*` registry、alignment parity registry、legacy `upstream` binary 或 Tcl/DejaGNU runner。

## 同步和门禁

```text
git fetch upstream
git merge upstream/main
BSC_JOBS=1 pixi run just contracts-update
BSC_JOBS=1 pixi run just plans-update
BSC_JOBS=1 pixi run just inventory-update
BSC_JOBS=1 pixi run just contracts-check
BSC_JOBS=1 pixi run just plans-check
BSC_JOBS=1 pixi run just plans-audit
BSC_JOBS=1 pixi run just inventory-check
BSC_JOBS=1 pixi run just test
```

Importer 输出必须确定性；连续生成不得产生 diff。每份 plan 记录 origin 和 fixture SHA-256，防止 upstream 变化后静默运行旧契约。

## 缓存与临时文件

- scenario result cache：`.pixi/cache/rust-tests/scenario-results/v1`（仅保存 assertion snapshots 与完成标记）

- disposable work/artifact：`.pixi/tmp/rust-test-*`
- 长测试日志：`.pixi/tmp/benchmarks/`
- Rust/C++ compiler cache：由 Pixi-managed `sccache` 统一提供；用户已有的 `RUSTC_WRAPPER`、`SCCACHE_DIR` 和 `SCCACHE_CACHE_SIZE` 优先

缓存命中只提供只读 assertion snapshots；普通 `.bo/.ba/object/executable` 和未被断言引用的 Verilog/C++ 产物不会进入持久缓存。Verilog、C++ 或 VCD 若本身是 assertion 的被测对象，则只保存对应断言时刻的快照。可用 `BSC_TEST_CACHE=0` 或 `just test-cold` 做冷验证，用 `just test-prune` 清理中断运行的临时目录。

## 当前基线

Test Plan schema v10：

```text
860 plans
584 complete / 276 blocked
4951 scenarios / 5055 stages / 20361 operations
2027 diagnostics
```

最近一次单线程 Windows warm-cache 全量验证：

```text
584 complete plans
2895 stages passed
5 skipped
0 scenarios failed
2837 scenario result cache hits
```

动态剩余数量以 `BSC_JOBS=1 pixi run just inventory-check` 和 [`REMAINING.md`](REMAINING.md) 为准，不在手写文档中复制第二套统计。
