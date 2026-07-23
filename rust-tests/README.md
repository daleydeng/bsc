# BSC Rust 测试层

这是一个独立的 Rust crate，用来承载 Cargo 原生 harness 单测、`testsuite/bsc.scheduler/sat` 的 Z3 scheduler contract tests，以及逐步迁移 upstream testsuite 的自定义动态 runner。`sha2` 用于稳定的内容寻址缓存，`regex` 用于跨平台 legacy golden 归一化。

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
pixi run just test-upstream   # 对齐检查后运行全部已迁移 upstream contracts
pixi run just test-upstream b1493 # 只运行名称包含 b1493 的 contract
```

也可以执行 `pixi run cargo test --manifest-path rust-tests/Cargo.toml`，但推荐使用项目任务，以便统一并发数和 Cargo target 目录。直接使用系统 `cargo test` 时，调用者还必须自行确保 `z3` 可从 `PATH` 找到。

`test-alignment` 同时报告来源脚本覆盖和 contract 覆盖。contract inventory 统计全仓库中可静态识别的 compile、Bluesim、Icarus 与 scheduler contract；Tcl 循环、自定义 helper 或多阶段流程不会被假装静态展开，而是计入“需要动态或自定义 Tcl 分析”的脚本数。

默认 `test` 对成功的 simulation generation workspace 使用 SHA-256 内容寻址缓存，目录为 `.pixi/cache/rust-tests/simulation-generation/v1`。key 包含当前 BSC 可执行文件、`inst/lib`、全部 fixture、generation argv 和关键环境；失败或超时不会写入缓存。缓存命中时仍会重新执行 BSC link、Bluesim/Icarus simulation 和 golden compare，因此只跳过最慢的 generation。

Simulation 声明以 `SimulationScenario` 为聚合根，并显式包含一个或多个 `SimulationContract`；不再把 backend case 平铺后于运行时猜测兼容分组。`BackendSpecific(backend)` 对应独立生成，`SharedElaboration` 对应 upstream `test_c_veri_*` 的一次 `-verilog -elab`。每个 scenario 只查询或发布一次 generation cache；scenario→contract 的运行期工作区克隆优先使用 Rust 标准库 `std::fs::hard_link`，文件系统不支持、跨卷或权限不足时自动回退 `fs::copy`，随后在隔离目录分别 link、simulate、比较 golden 和检查 VCD。持久化 cache 恢复始终使用真实复制，contract 不会 hardlink 到 cache entry；生成失败会一次性报告到该 scenario 的全部已启用 contract。

908 个 compile contract 和 24 个 scheduler contract 使用统一的 BSC result cache，目录为 `.pixi/cache/rust-tests/bsc-results/v1`。key 除 toolchain、fixture、argv 和环境外还包含实际 `z3.exe` 的内容指纹；只有已经通过对应诊断、产物和 golden 检查的原始 BSC 结果才会发布。命中后仍重新执行 Rust 侧的 exit status、diagnostic、产物、normalization 和 golden 检查，不缓存最终 pass/fail。

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

该值同时控制 Cargo 编译任务数、Rust test harness 线程数和普通 upstream runner worker 数。`ResourceClass::Heavy` scenario 会在普通队列结束后进入单 worker 队列，避免重型 elaboration 彼此竞争，也避免同一 dual-backend scenario 重复生成。

Bluesim 与 Verilog backend case 默认启用。设置 `CTEST=0` 或 `VTEST=0` 会将对应 backend 的 case 明确报告为 `SKIPPED`，其他 case 仍正常运行；汇总分别显示 passed/skipped/failed，且只有 failed 会令 runner 返回非零退出码：

```powershell
$env:CTEST = 0  # 可选：禁用 Bluesim case
$env:VTEST = 0  # 可选：禁用 Verilog/Icarus case
pixi run just test-upstream
```

Upstream 动态 runner 支持列出 case、substring filter、精确匹配和固定 worker 数：

```sh
pixi run just test-upstream --list
pixi run just test-upstream b1493
pixi run just test-upstream b1493::Bug1493_Bad.bsv --exact
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
- `.exp` 中已支持的 compile/simulation API 调用按 source 和 backend 展开后，必须与 Rust 注册表逐项、逐数量一致；simulation helper 的 shared/backend-specific generation strategy 也必须逐项一致。
- `compare_file` 必须与 Rust golden 声明一致，所有声明的 source、fixture 和 expected 文件必须存在。
- Rust scheduler case 列表必须与 `sat.exp` 的 `set sources` 顺序和内容一致，且每个 BSV/Yices expected 必须存在。
- 全局 Rust contract name 与 simulation scenario name 必须分别唯一；scenario 的 source、expected、backend requirement、VCD expectation、generated modules、timeout 和 fixture 声明必须满足结构约束。
- 递归统计整个 `testsuite` 的 `.exp` 测试来源，排除 `config/unix.exp`、`lib/bsc.exp` 和 `site.exp` 三个 harness 文件，并报告已迁移与剩余脚本数。迁移完成前，剩余项只报告覆盖率，不导致失败。

`test-upstream` 和默认 `test` 已自动先运行 alignment 与 `inventory-check`，因此 upstream 新增、删除或改名受支持的 Tcl 调用、Rust 完成新迁移但忘记更新剩余清单时，都会在执行较慢的 BSC tests 前快速失败。

## 工作目录与产物

每个测试进程生成至少包含 pid 和时间戳的唯一 `<run-id>`，因此两个 runner 并发时不会互相清理目录：

- scheduler 工作目录：`.pixi/tmp/rust-test-work/scheduler-sat/<run-id>/<case>`
- scheduler 日志与 diff：`.pixi/tmp/rust-test-artifacts/scheduler-sat/<run-id>/<case>`
- upstream 工作目录：`.pixi/tmp/rust-test-work/upstream/<run-id>/<case>`
- upstream 日志与 diff：`.pixi/tmp/rust-test-artifacts/upstream/<run-id>/<case>`

每个 case/scenario 只清理自己当前 run-id 下的目录。Scheduler 的 `bsc-schedule.log` 和 compile case 的 `bsc.log` 均包含命令、工作目录、BSC stdout/stderr、退出状态和耗时；simulation scenario 的 generation artifact 写 `compile.log`，各 contract 的隔离 artifact 分别写 `link.log`、`simulation.log` 和 VCD 产物。Compile case 另将原始编译输出写为 `<source>.bsc-out`，任何 golden mismatch 都写入 `golden.diff`。

BSC 运行时设置：

- `BLUESPECDIR=<项目根>/inst/lib`
- `BSCTEST=1`
- 普通 scenario 超时 300 秒，heavy scenario 超时 600 秒；Windows 上通过 `taskkill /PID <pid> /T /F` 清理整个进程树

stdout/stderr 直接写文件而不是 pipe，避免子进程输出较多时发生 pipe deadlock。

## 覆盖范围

当前总计覆盖 **1305 个 upstream contract**：24 个 Cargo 原生 Z3 scheduler contract，以及由自定义动态 runner 执行的 908 个 compile contract 和 373 个 simulation contract。来源覆盖为 310/860；完整的动态统计始终以 `test-alignment` 和自动生成的 [`REMAINING.md`](REMAINING.md) 为准。

24 个 Z3 scheduler case：

`BoolTest`、`AddTest`、`MultTest`、`DivTest`、`RemTest`、`ShiftRTest`、`ShiftRATest`、`ShiftLTest`、`LessThanSTest`、`LessThanTest`、`ZextTest`、`SextTest`、`IteTest`、`TruncateTest`、`ShiftRATest2`、`ArraySelectTest`、`CaseTest`、`ArraySelectShortIndexTest`、`ArraySelectLongIndexTest`、`ArraySelectImplCondTest`、`ParamBoolTest`、`ParamBitsTest`、`Word64Test`、`SplitTupleMethodTest`。

每个 case 将原始 `<case>.bsv` 复制并重命名为 `<case>_sat-z3.bsv`，使用迁移前相同的参数：

```text
-sat-z3 -no-show-timestamps -no-show-version -u -resource-simple
-show-schedule -dschedule -dresources -dvschedinfo -verilog <case>_sat-z3.bsv
```

测试检查 BSC 退出成功、生成 `<case>_sat-z3.bo`，并复用 upstream 的 `<case>_sat-yices.bsv.bsc-sched-out.expected`。比较前会统一 CRLF/CR、应用 `diff -b` 风格的空白归一、归一化 `__h数字`/`__d数字` 生成 ID，并把 `_sat-stp`、`_sat-yices`、`_sat-z3` 后缀统一为 `_sat-SOLVER`。

Upstream runner 当前完整覆盖 309 个普通 `.exp` 脚本、1281 个动态 contract。Compile pipeline 展开为 908 个独立 contract；simulation pipeline 声明 235 个 generation scenario，并展开为 373 个 backend contract。加上 24 个 Z3 scheduler contract 后，来源覆盖为 310/860，静态可识别 contract 覆盖为 1305/4368，另有 233 个脚本需要动态或自定义 Tcl 分析。

Compile 数据模块在文件末尾导出 `CASES`，simulation 数据模块导出 `SCENARIOS`；`src/upstream/cases_compile.rs` 和 `src/upstream/cases_simulation.rs` 各用一个宏列表同时生成 `mod` 声明与模块描述表，再通过 `OnceLock` 一次性展平。新增声明只需修改所属模块，中央不逐项维护 1281 个 contract 引用；执行顺序固定为中央模块名顺序、模块内 scenario/case 顺序和 scenario 内 contract 顺序。Frontend mode 使用 `-no-show-timestamps -no-show-version`、可选 `-u` 和 source；Verilog mode 对齐 `bsc_compile_verilog`，使用 `-no-show-timestamps -no-show-version -u -verilog`，仅在 module 非空时追加 `-g <module>`。Pass 检查 `<stem>.bo`，Fail 检查非零退出，带诊断的 Fail 精确统计行尾 `(TAG)`。

`src/upstream.rs` 保留公共数据模型、注册表、CLI 和跨 pipeline helper；`src/upstream/compile.rs`、`src/upstream/simulation.rs`、`src/upstream/runner.rs` 分别承载编译管线、仿真管线和并发执行计划，单元测试集中在 `src/upstream/tests.rs`。

Simulation runner 直接执行声明的 scenario，不再计算 generation compatibility。`BackendSpecific(Bluesim/Icarus)` 分别使用 `-sim` 或 `-verilog`；`SharedElaboration` 对齐 upstream `test_c_veri_*`，一次执行 `-verilog -elab` 并复用 `.ba`/`.v` 产物。生成工作区只做一次 cache lookup，随后优先 hardlink、失败时 copy 到每个 backend contract 的隔离工作区；Bluesim 与 Icarus 再分别 link、simulate、比较 backend golden。VCD contract 还会验证 Bluesim VCD 模式输出一致性或 Icarus 非空 VCD smoke。原生 Windows 上 Bluesim launcher 使用 `sh`，Icarus 产物由 `vvp` 启动。Icarus 输出应用 legacy 噪声过滤；runner 还会读取 `iverilog -V`，按 upstream exclusion 显式跳过版本能力不足的 contract。

迁移历史见 [`MIGRATION.md`](MIGRATION.md)，已经人工核对的下一批候选与阻塞原因见 [`NEXT.md`](NEXT.md)，由 alignment 同源生成的全部剩余 550 个脚本见 [`REMAINING.md`](REMAINING.md)。

## 边界与后续方向

这不是完整的 upstream testsuite，也不是 Tcl 解释器；当前只迁移明确建模的 contract case，不替代尚未迁移的编译器、模拟器或后端测试。

后续可以把这个 crate 扩展为 Haskell 与 Rust 两套实现共享的 contract test 层：保持同一输入、参数、归一化规则和 golden contract，分别运行两种实现并比较可观察行为，而不是把测试绑定到某一种内部实现。
