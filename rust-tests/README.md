# BSC Rust 测试层

这是一个独立、零第三方依赖的 Rust crate，用来承载 Cargo 原生 harness 单测、`testsuite/bsc.scheduler/sat` 的 Z3 scheduler contract tests，以及逐步迁移 upstream testsuite 的自定义动态 runner。

## 运行

先在项目根目录构建 BSC，确保存在 `inst/bin/core/bsc.exe`（Windows）或 `inst/bin/core/bsc`（其他平台），并且 `inst/lib` 已生成。推荐通过项目的 Pixi 环境运行，以便把 `z3prover` 加入 `PATH`：

```sh
pixi run just test           # 运行 Rust 自测和全部已迁移 contract tests
pixi run just test-z3        # 只运行 24 个 Z3 scheduler tests
pixi run just test-upstream  # 只运行动态迁移的 upstream cases
```

也可以执行 `pixi run cargo test --manifest-path rust-tests/Cargo.toml`，但推荐使用项目任务，以便统一并发数和 Cargo target 目录。直接使用系统 `cargo test` 时，调用者还必须自行确保 `z3` 可从 `PATH` 找到。

默认测试 `inst/bin/core/bsc.exe`。未来验证另一份实现时可以覆盖被测程序：

```powershell
$env:BSC_UNDER_TEST = "target/release/bsc-rs.exe"
pixi run just test
```

相对路径按项目根目录解析。

Rust 原生测试保留给 helper 单测和 scheduler，每个 scheduler case 都是独立的 `#[test]`，可按名称过滤，例如：

```sh
pixi run cargo test --manifest-path rust-tests/Cargo.toml scheduler_sat_bool_test
pixi run cargo test --manifest-path rust-tests/Cargo.toml scheduler_sat_array_select
pixi run cargo test --manifest-path rust-tests/Cargo.toml normalization_
```

如需限制 BSC 并发数，可在 PowerShell 中设置：

```powershell
$env:BSC_JOBS = 4
pixi run just test
```

该值同时控制 Cargo 编译任务数、Rust test harness 线程数和 upstream runner worker 数。

Bluesim 与 Verilog backend case 默认启用。设置 `CTEST=0` 或 `VTEST=0` 会将对应 backend 的 case 明确报告为 `SKIPPED`，其他 case 仍正常运行；汇总分别显示 passed/skipped/failed，且只有 failed 会令 runner 返回非零退出码：

```powershell
$env:CTEST = 0  # 可选：禁用 Bluesim case
$env:VTEST = 0  # 可选：禁用 Verilog/Icarus case
pixi run just test-upstream
```

Upstream 动态 runner 支持列出 case、substring filter、精确匹配和固定 worker 数：

```sh
pixi run cargo run --manifest-path rust-tests/Cargo.toml --bin upstream -- --list
pixi run cargo run --manifest-path rust-tests/Cargo.toml --bin upstream -- b1493
pixi run cargo run --manifest-path rust-tests/Cargo.toml --bin upstream -- b1493::Bug1493_Bad.bsv --exact --test-threads 1
```

## 工作目录与产物

每个测试进程生成至少包含 pid 和时间戳的唯一 `<run-id>`，因此两个 runner 并发时不会互相清理目录：

- scheduler 工作目录：`.pixi/tmp/rust-test-work/scheduler-sat/<run-id>/<case>`
- scheduler 日志与 diff：`.pixi/tmp/rust-test-artifacts/scheduler-sat/<run-id>/<case>`
- upstream 工作目录：`.pixi/tmp/rust-test-work/upstream/<run-id>/<case>`
- upstream 日志与 diff：`.pixi/tmp/rust-test-artifacts/upstream/<run-id>/<case>`

每个 case 只清理自己当前 run-id 下的目录。Scheduler 的 `bsc-schedule.log` 和 compile case 的 `bsc.log` 均包含命令、工作目录、BSC stdout/stderr、退出状态和耗时；simulation case 分别写 `compile.log`、`link.log`、`simulation.log`。Compile case 另将原始编译输出写为 `<source>.bsc-out`，任何 golden mismatch 都写入 `golden.diff`。

BSC 运行时设置：

- `BLUESPECDIR=<项目根>/inst/lib`
- `BSCTEST=1`
- 超时 300 秒；Windows 上通过 `taskkill /PID <pid> /T /F` 清理整个进程树

stdout/stderr 直接写文件而不是 pipe，避免子进程输出较多时发生 pipe deadlock。

## 覆盖范围

当前总计覆盖 **212 个独立 contract case**：24 个 Cargo 原生 Z3 scheduler case，加上由自定义动态 runner 执行的 188 个 upstream case（126 compile + 62 simulation，来自 49 个 `.exp` 脚本）。

24 个 Z3 scheduler case：

`BoolTest`、`AddTest`、`MultTest`、`DivTest`、`RemTest`、`ShiftRTest`、`ShiftRATest`、`ShiftLTest`、`LessThanSTest`、`LessThanTest`、`ZextTest`、`SextTest`、`IteTest`、`TruncateTest`、`ShiftRATest2`、`ArraySelectTest`、`CaseTest`、`ArraySelectShortIndexTest`、`ArraySelectLongIndexTest`、`ArraySelectImplCondTest`、`ParamBoolTest`、`ParamBitsTest`、`Word64Test`、`SplitTupleMethodTest`。

每个 case 将原始 `<case>.bsv` 复制并重命名为 `<case>_sat-z3.bsv`，使用迁移前相同的参数：

```text
-sat-z3 -no-show-timestamps -no-show-version -u -resource-simple
-show-schedule -dschedule -dresources -dvschedinfo -verilog <case>_sat-z3.bsv
```

测试检查 BSC 退出成功、生成 `<case>_sat-z3.bo`，并复用 upstream 的 `<case>_sat-yices.bsv.bsc-sched-out.expected`。比较前会统一 CRLF/CR、应用 `diff -b` 风格的空白归一、归一化 `__h数字`/`__d数字` 生成 ID，并把 `_sat-stp`、`_sat-yices`、`_sat-z3` 后缀统一为 `_sat-SOLVER`。

Upstream runner 当前完整覆盖 49 个 `.exp` 脚本。Compile pipeline 展开为 126 个独立 case：36 个 Pass、32 个普通 Fail、58 个精确诊断 Fail，其中 34 个 case 还比较 golden；mode 为 81 frontend、45 Verilog。Simulation pipeline 展开为 62 个独立 case：31 Bluesim、31 Icarus，覆盖 `dynamic`、数组 bounds select/update 和 Gearbox。Phase 1 的 9 个 case name 保持不变。

Case 数据位于 `src/upstream/cases_compile.rs` 和 `src/upstream/cases_simulation.rs`，并按 testsuite 目录拆分。Frontend mode 使用 `-no-show-timestamps -no-show-version`、可选 `-u` 和 source；Verilog mode 对齐 `bsc_compile_verilog`，使用 `-no-show-timestamps -no-show-version -u -verilog`，仅在 module 非空时追加 `-g <module>`。Pass 检查 `<stem>.bo`，Fail 检查非零退出，带诊断的 Fail 精确统计行尾 `(TAG)`。

Simulation case 分别执行 generate、link、simulate：Bluesim 使用 `-sim`，Verilog 使用 `-verilog -vsim iverilog`。原生 Windows 上 BSC 生成的 Bluesim launcher 是 `sh` 脚本，Icarus 产物是 `vvp` 字节码，runner 会选择正确启动器并将 `inst/bin/core` 前置到子进程 `PATH`。Icarus 输出应用 legacy 噪声过滤。所有 golden 均按 Tcl `compare_file` 的 `diff -b` 语义比较，并忽略包含 `SystemC` 或 `dumpfile parameter` 的行。

完整迁移盘点和路线见 [`MIGRATION.md`](MIGRATION.md)。

## 边界与后续方向

这不是完整的 upstream testsuite，也不是 Tcl 解释器；当前只迁移明确建模的 contract case，不替代尚未迁移的编译器、模拟器或后端测试。

后续可以把这个 crate 扩展为 Haskell 与 Rust 两套实现共享的 contract test 层：保持同一输入、参数、归一化规则和 golden contract，分别运行两种实现并比较可观察行为，而不是把测试绑定到某一种内部实现。
