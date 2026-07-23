# BSC Rust 测试层

这是一个独立的 Rust crate，用来承载 Cargo 原生 harness 单测、`testsuite/bsc.scheduler/sat` 的 Z3 scheduler contract tests，以及逐步迁移 upstream testsuite 的自定义动态 runner。唯一直接依赖 `sha2` 用于测试 generation cache 的稳定内容寻址。

## 运行

先在项目根目录构建 BSC，确保存在 `inst/bin/core/bsc.exe`（Windows）或 `inst/bin/core/bsc`（其他平台），并且 `inst/lib` 已生成。推荐通过项目的 Pixi 环境运行，以便把 `z3prover` 加入 `PATH`。

Windows 上的 Icarus 来自已有的 OSS CAD Suite，而不是 conda-forge。首次使用时将本机安装目录记录到被 Git 忽略的 `.pixi/oss-cad-suite-root.txt`：

```powershell
pixi run just configure-oss-cad-suite D:\software\oss-cad-suite
```

也可以设置 `OSS_CAD_SUITE_ROOT` 或 OSS CAD Suite 自带的 `YOSYSHQ_ROOT` 环境变量。测试只把该安装的 `bin` 和 `lib` 前置到 PATH，不会加载其 Python、Qt 或 GTK 环境。配置完成后运行：

```sh
pixi run just test            # 使用 BSC 内容缓存和 C++ compiler cache 运行全部 contract tests
pixi run just test-cold       # 禁用全部缓存，执行完整冷验证
pixi run just test-alignment  # 只检查 Rust 声明是否仍与 testsuite 对齐
pixi run just inventory-check # 检查完整剩余清单是否与当前迁移状态一致
pixi run just inventory-update # 迁移后重建 rust-tests/REMAINING.md
pixi run just test-z3         # 只运行 24 个 Z3 scheduler tests
pixi run just test-upstream   # 对齐检查后运行动态迁移的 upstream cases
```

也可以执行 `pixi run cargo test --manifest-path rust-tests/Cargo.toml`，但推荐使用项目任务，以便统一并发数和 Cargo target 目录。直接使用系统 `cargo test` 时，调用者还必须自行确保 `z3` 可从 `PATH` 找到。

`test-alignment` 同时报告来源脚本覆盖和 contract 覆盖。contract inventory 统计全仓库中可静态识别的 compile、Bluesim、Icarus 与 scheduler contract；Tcl 循环、自定义 helper 或多阶段流程不会被假装静态展开，而是计入“需要动态或自定义 Tcl 分析”的脚本数。

默认 `test` 对成功的 simulation generation workspace 使用 SHA-256 内容寻址缓存，目录为 `.pixi/cache/rust-tests/simulation-generation/v1`。key 包含当前 BSC 可执行文件、`inst/lib`、全部 fixture、generation argv 和关键环境；失败或超时不会写入缓存。缓存命中时仍会重新执行 BSC link、Bluesim/Icarus simulation 和 golden compare，因此只跳过最慢的 generation。

422 个 compile contract 和 24 个 scheduler contract 使用统一的 BSC result cache，目录为 `.pixi/cache/rust-tests/bsc-results/v1`。key 除 toolchain、fixture、argv 和环境外还包含实际 `z3.exe` 的内容指纹；只有已经通过对应诊断、产物和 golden 检查的原始 BSC 结果才会发布。命中后仍重新执行 Rust 侧的 exit status、diagnostic、产物、normalization 和 golden 检查，不缓存最终 pass/fail。

Bluesim link 中的生成 C++ 编译默认通过 Pixi 管理的 `ccache` 执行，缓存位于 `.pixi/cache/ccache`；最终链接、simulation 和 golden compare 仍会真实执行。`pixi run just ccache-stats` 可查看统计，`pixi run just ccache-clear` 可清空该层缓存。显式设置 `CXX` 时任务会尊重调用者配置；`test-cold` 会同时禁用 generation cache、BSC result cache 和 `ccache`，保留完整无缓存验证入口。

GitHub Actions 的原生 Windows job 会分别持久化 `.pixi/cache/rust-tests` 和 `.pixi/cache/ccache`。每个 commit 使用独立的不可变 cache key，并从相同 `pixi.lock` 的最近快照恢复；恢复后的 Rust entry 仍必须通过内部 BSC、`inst/lib`、fixture、argv、环境和 Z3 内容指纹校验，因此 CI cache 只减少重复计算，不放宽 contract。

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

该值同时控制 Cargo 编译任务数、Rust test harness 线程数和普通 upstream runner worker 数。标记为重型的 case 会在普通队列结束后进入独立队列，最多使用 2 个 worker，避免与 16 路普通 elaboration 争用资源。

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

## 来源与对齐检查

每个 `cases_compile/`、`cases_simulation/` 模块顶部都必须以严格格式的 `//! Origin:` 或 `//! Origins:` 显式列出全部原始 `.exp`，不接受通配符或目录模板；具体来源还会由模块内每个 case 的 `fixture_dir` 反向推导并双向核对。模块按稳定的来源范围和 contract 形态命名，迁移批次编号只记录在 `MIGRATION.md`，不会进入 Rust module 名。Scheduler 集成测试也直接标注来源 `testsuite/bsc.scheduler/sat/sat.exp`。

运行快速对齐检查：

```sh
pixi run just test-alignment
```

`alignment` 不运行 BSC，而是检查：

- `cases_compile/` 与 `cases_simulation/` 中的磁盘模块文件必须和中央单一宏注册表完全一致，禁止孤立文件、缺失模块、空模块和重复注册。
- 模块名必须是稳定的 ASCII snake_case，且不能包含 `batch`、`large`、`other`、`four`、`five` 等迁移过程词。
- 每个模块声明的 `Origin(s)` 必须与该模块 case 实际推导出的 `.exp` 来源集合完全一致；来源路径必须存在、不得重复，也不能使用模板。
- 每个 fixture 目录当前必须恰好包含一个来源 `.exp`；若未来同目录有多个脚本，检查会要求先增加显式 origin 元数据，禁止猜测。
- `.exp` 中已支持的 compile/simulation API 调用按 source 和 backend 展开后，必须与 Rust 注册表逐项、逐数量一致。
- `compare_file` 必须与 Rust golden 声明一致，所有声明的 source、fixture 和 expected 文件必须存在。
- Rust scheduler case 列表必须与 `sat.exp` 的 `set sources` 顺序和内容一致，且每个 BSV/Yices expected 必须存在。
- 全局 Rust case name 必须唯一。
- 递归统计整个 `testsuite` 的 `.exp` 测试来源，排除 `config/unix.exp`、`lib/bsc.exp` 和 `site.exp` 三个 harness 文件，并报告已迁移与剩余脚本数。迁移完成前，剩余项只报告覆盖率，不导致失败。

`test-upstream` 和默认 `test` 已自动先运行 alignment 与 `inventory-check`，因此 upstream 新增、删除或改名受支持的 Tcl 调用、Rust 完成新迁移但忘记更新剩余清单时，都会在执行较慢的 BSC tests 前快速失败。

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

当前总计覆盖 **299 个独立 contract case**：24 个 Cargo 原生 Z3 scheduler case，加上由自定义动态 runner 执行的 275 个 upstream case（159 compile + 116 simulation，来自 64 个 `.exp` 脚本）。

24 个 Z3 scheduler case：

`BoolTest`、`AddTest`、`MultTest`、`DivTest`、`RemTest`、`ShiftRTest`、`ShiftRATest`、`ShiftLTest`、`LessThanSTest`、`LessThanTest`、`ZextTest`、`SextTest`、`IteTest`、`TruncateTest`、`ShiftRATest2`、`ArraySelectTest`、`CaseTest`、`ArraySelectShortIndexTest`、`ArraySelectLongIndexTest`、`ArraySelectImplCondTest`、`ParamBoolTest`、`ParamBitsTest`、`Word64Test`、`SplitTupleMethodTest`。

每个 case 将原始 `<case>.bsv` 复制并重命名为 `<case>_sat-z3.bsv`，使用迁移前相同的参数：

```text
-sat-z3 -no-show-timestamps -no-show-version -u -resource-simple
-show-schedule -dschedule -dresources -dvschedinfo -verilog <case>_sat-z3.bsv
```

测试检查 BSC 退出成功、生成 `<case>_sat-z3.bo`，并复用 upstream 的 `<case>_sat-yices.bsv.bsc-sched-out.expected`。比较前会统一 CRLF/CR、应用 `diff -b` 风格的空白归一、归一化 `__h数字`/`__d数字` 生成 ID，并把 `_sat-stp`、`_sat-yices`、`_sat-z3` 后缀统一为 `_sat-SOLVER`。

Upstream runner 当前完整覆盖 238 个普通 `.exp` 脚本、594 个动态 case。Compile pipeline 展开为 422 个独立 case：165 个普通 Pass、6 个带精确诊断的 Pass、100 个普通 Fail、151 个精确诊断 Fail，其中 128 个 case 还比较 golden；mode 为 304 frontend、118 Verilog。Simulation pipeline 展开为 172 个独立 case：86 Bluesim、86 Icarus。加上 24 个 Z3 scheduler contract 后共迁移 618 个 contract；计入 `sat.exp`，来源覆盖为 239/860，静态可识别 contract 覆盖为 618/4161，另有 250 个脚本需要动态或自定义 Tcl 分析。

每个具体数据模块在自己的文件末尾导出模块级 `CASES` slice；`src/upstream/cases_compile.rs` 和 `src/upstream/cases_simulation.rs` 只用一个宏列表同时生成 `mod` 声明与模块描述表，再通过 `OnceLock` 一次性展平。新增 case 只需修改所属模块，中央不再逐项维护 594 个常量引用；执行顺序固定为中央模块名顺序加模块内声明顺序。Frontend mode 使用 `-no-show-timestamps -no-show-version`、可选 `-u` 和 source；Verilog mode 对齐 `bsc_compile_verilog`，使用 `-no-show-timestamps -no-show-version -u -verilog`，仅在 module 非空时追加 `-g <module>`。Pass 检查 `<stem>.bo`，Fail 检查非零退出，带诊断的 Fail 精确统计行尾 `(TAG)`。

Simulation case 分别执行 generate、link、simulate：Bluesim 使用 `-sim`，Verilog 使用 `-verilog -vsim iverilog`。原生 Windows 上 BSC 生成的 Bluesim launcher 是 `sh` 脚本，Icarus 产物是 `vvp` 字节码，runner 会选择正确启动器并将 `inst/bin/core` 前置到子进程 `PATH`。Icarus 输出应用 legacy 噪声过滤；runner 还会读取 `iverilog -V`，按 upstream exclusion 显式跳过版本能力不足的 case。所有 golden 均按 Tcl `compare_file` 的 `diff -b` 语义比较，并忽略包含 `SystemC` 或 `dumpfile parameter` 的行。

迁移历史见 [`MIGRATION.md`](MIGRATION.md)，已经人工核对的下一批候选与阻塞原因见 [`NEXT.md`](NEXT.md)，由 alignment 同源生成的全部剩余 621 个脚本见 [`REMAINING.md`](REMAINING.md)。

## 边界与后续方向

这不是完整的 upstream testsuite，也不是 Tcl 解释器；当前只迁移明确建模的 contract case，不替代尚未迁移的编译器、模拟器或后端测试。

后续可以把这个 crate 扩展为 Haskell 与 Rust 两套实现共享的 contract test 层：保持同一输入、参数、归一化规则和 golden contract，分别运行两种实现并比较可观察行为，而不是把测试绑定到某一种内部实现。
