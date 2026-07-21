# Testsuite 全量迁移计划

## 目标与边界

本目录用 Rust 数据模型和可复用 helper 逐项迁移 upstream `testsuite` 的可观察测试契约。迁移对象是 case、fixture、命令参数、退出状态、产物、诊断和 golden 比较规则，**不是实现或嵌入一个 Tcl 解释器**。

Cargo 原生 test harness 继续承载 Rust helper 单测和已经迁移的 scheduler 静态 tests；数量较大、需要运行时发现/筛选的 upstream case 由 `src/bin/upstream.rs` 自定义动态 runner 承载。

只读取 `testsuite/` 中的 fixture 和 golden。每个 case 将声明的文件复制到独立 workspace 后运行，不在原 testsuite 目录中生成或修改文件。

## 静态盘点

当前对 release testsuite 的静态盘点基线为 **867 个脚本**：

| 类型 | 约数 | 特征 | 迁移方式 |
| --- | ---: | --- | --- |
| 机械型 | 约 700 | 标准 compile/simulate、退出状态、固定诊断或 golden | 声明式 case 数据 + 通用 pipeline |
| 模板型 | 136 | 少量参数组合、循环、后端矩阵或重复流程 | Rust 模板/生成器展开为独立动态 case |
| 专用型 | 31 | 自定义工具、复杂文件变换或特殊环境编排 | 按脚本建专用 helper，保留明确 contract |
| 合计 | 867 | release scripts | 分阶段迁移并持续校准盘点 |

“约 700”是由总数扣除 136 个模板型和 31 个专用型后的机械迁移规模，用于路线规划；后续迁移中若脚本分类变化，应同步更新此文档和覆盖统计。

## 分阶段路线

### Phase 1：compile pipeline 与动态 harness（已完成）

- 建立零第三方依赖的 upstream 动态 runner。
- 建立可扩展的 `CompileCase`、`CompileExpectation`、diagnostic 和可选 golden 模型。
- 对齐 `testsuite/config/unix.exp` 的通用 compile 行为：独立 cwd、fixture staging、BSC 参数、`.bo`、非零退出、诊断计数及 legacy golden diff。
- 支持 `--list`、substring filter、`--exact`、`--test-threads N` 和固定 worker queue。
- workspace/artifact 使用进程级唯一 run-id，避免并发 runner 互删。

### Phase 2：机械型 compile 批量迁移（第四批已完成）

- 已逐份核对并迁移 48 个包含 compile contract 的 `.exp` 脚本，按每次公共 compile API 调用展开为 126 个动态 case。
- 覆盖 frontend/Verilog compile mode、compile pass/fail、diagnostic kind/tag/count 与默认 golden；当前 case 全部使用空 options 和 `nodeps=0`。
- Case 数据从公共 runner 中迁出，按 `bluespec_inc` pass、diagnostic fail、golden/mixed 和其他 testsuite 目录拆分。
- 后续机械批次继续启用模型中预留的 `options` 和 `nodeps`，并增加多 fixture、include 路径、额外预期产物等小型通用 contract。

### Phase 3：模板型与后端矩阵（backend capability/policy 已启动）

- 将 Tcl 循环和参数矩阵显式展开为可单独筛选、单独报告的动态 case。
- 已增加一等 Verilog compile mode、Bluesim/Icarus simulation pipeline、`BluesimEnabled`/`VerilogEnabled` requirement、`CTEST`/`VTEST` policy 和显式 skip 结果；generate、link、simulate 共享进程、日志、超时、归一化和 diff 基础设施。
- 保持每个展开 case 的 fixture/workspace 隔离，避免矩阵并行时共享产物。

### Phase 4：专用脚本

- 对 31 个专用脚本逐一定义 contract，不做 Tcl 语法兼容层。
- 优先提取可复用的文件处理、外部工具调用和输出检查 helper；确实唯一的流程保留小型专用 runner 函数。
- 对平台条件、可选工具和资源需求使用显式 skip/unsupported 结果，不静默降低检查强度。

### Phase 5：全量切换与守护

- 对迁移前后结果做批量双跑，核对 pass/fail、诊断数量、golden 和生成物。
- 建立覆盖清单和 CI 分片，区分快速 compile、scheduler、backend、simulation 与专用测试。
- 只有在对应 contract 已迁移并双跑稳定后，才考虑从 release 流程移除原 Tcl case。

## Phase 1 已迁移清单

8 个 `.exp` 共展开为 9 个独立动态 case：

| `.exp` | 动态 case | 预期 | 额外检查 |
| --- | --- | --- | --- |
| `b600/b600.exp` | `b600::Bug600.bsv` | Pass | 生成 `Bug600.bo` |
| `b267/b267.exp` | `b267::Bug267.bs` | Pass | 生成 `Bug267.bo` |
| `b1040/b1040.exp` | `b1040::Bug1040.bsv` | Fail | `Error` / `P0127` / 1 |
| `b417/b417.exp` | `b417::Bug417.bsv` | Fail | `Error` / `S0007` / 1 |
| `b492/b492.exp` | `b492::Bug492_1.bs` | Fail | `Error` / `T0046` / 1 |
| `b1586/b1586.exp` | `b1586::Bug1586.bsv` | Fail | `Bug1586.bsv.bsc-out.expected` |
| `b269/b269.exp` | `b269::Bug269.bsv` | Fail | `Error` / `P0070` / 1；golden |
| `b1493/b1493.exp` | `b1493::Bug1493.bsv` | Pass | 生成 `Bug1493.bo` |
| `b1493/b1493.exp` | `b1493::Bug1493_Bad.bsv` | Fail | `Error` / `T0020` / 1 |

截至 Phase 1，Rust 测试层共覆盖 **33 个独立 case**：上述 9 个 upstream compile case，加上 24 个 Z3 scheduler case。

## Phase 2 compile 批次

第一批完整迁移 40 个 `.exp` 脚本，展开为 44 个动态 compile case：

| `.exp` 分类 | 脚本数 | 动态 case 数 | Contract |
| --- | ---: | ---: | --- |
| `compile_pass` | 18 | 18 | 编译成功并生成 `.bo` |
| `compile_fail_error` | 10 | 11 | 编译失败并精确匹配 Error tag；`b580` 展开 2 个 case |
| fail + golden | 9 | 9 | 7 个普通 Fail、2 个带 tag Fail；全部比较默认 golden |
| mixed，每份 2 次调用 | 3 | 6 | `b1493`、`moduletype`、`b557` 各 1 Pass + 1 tagged Fail |
| **合计** | **40** | **44** | 21 Pass、7 普通 Fail、16 tagged Fail、9 golden |

目录覆盖包括 `testsuite/bsc.bugs/bluespec_inc`、`testsuite/bsc.bugs/github`、`testsuite/bsc.interra/{bugs,messages}` 和 `testsuite/bsc.syntax/bsv05/moduletype`。`b580` 也是双调用脚本，因此 40 个脚本相对 44 个 case 的四个增量分别来自 `b580` 与三份 mixed 脚本。

Phase 1 的 9 个 case name 原样保留，以兼容已有 substring/exact filter。新增非 `bluespec_inc` case 使用包含 testsuite 目录语义的稳定 ID。所有 case 显式声明 source fixture；9 个 golden case 同时显式声明对应 `<source>.bsc-out.expected` fixture。

### Phase 2 第二批

第二批从真实 `.exp` 逐项核对并完整迁移 3 个目录脚本，共新增 23 个动态 compile case：

| `.exp` | 动态 case 数 | Contract |
| --- | ---: | --- |
| `bsc.interra/messages/ENotField/ENotField.exp` | 4 | 4 个普通 Fail，均显式声明 source + `<source>.bsc-out.expected` fixtures 并比较 golden |
| `bsc.misc/attrErrors/attrErrors.exp` | 10 | 1 Pass + 9 个 Error/tag/count=1 Fail |
| `bsc.typechecker/kind/inferkinds/inferkinds.exp` | 9 | 7 Pass + 2 个 Error/tag/count=1 Fail |
| **本批合计** | **23** | 8 Pass、4 普通 Fail、11 tagged Fail、4 golden |

### Phase 2 第三批与首个 backend capability

完整迁移 `bsc.syntax/bsv05/underscore/underscore.exp`，新增 17 个动态 case：4 个 frontend Pass、10 个 Error/tag/count=1 Fail 和 3 个 Verilog Pass。3 个 Verilog case 的 module 均为空，argv 对齐 `bsc_compile_verilog`，并按 `check_intermediate_files` 检查 source stem 对应的 `.bo`。

本批同时引入一等 `CompileMode` 和 `Requirement`：普通 frontend compile 不借用 `options: ["-verilog"]`，Verilog mode 负责 `-u -verilog` 及非空 module 的 `-g`；`VTEST` policy 默认启用，`VTEST=0` 时 3 个 Verilog case 明确 SKIP，另外 81 个 case 仍运行。结果和汇总区分 passed/skipped/failed，只有 failed 导致非零退出码。

Phase 2 前三批累计迁移 **44 个 `.exp` 脚本、84 个动态 compile case**：36 Pass、11 普通 Fail、37 tagged Fail、13 golden；mode 为 81 frontend、3 Verilog。

### Phase 2 第四批：Verilog 失败 contract

完整迁移 `bsc.evaluator/dynamic/{dynamic.exp,errors/dynamic_errors.exp}` 与 `bsc.arrays/bounds/{select,update}` 中的 42 个 Verilog fail contract：21 个 dynamic error golden、20 个 bounds `S0015` 诊断和 1 个 `DynamicIntegerFail` `T0051` 诊断。至此 compile pipeline 累计 **48 个脚本、126 个 case**：36 Pass、32 普通 Fail、58 tagged Fail、34 golden；mode 为 81 frontend、45 Verilog。所有 compile case 继续使用空 options 和 `nodeps=0`。

### Phase 3 第一批：Bluesim/Icarus simulation matrix

新增 `SimulationCase`、`SimulationBackend` 与统一 `UpstreamCase` runner，按 backend 独立执行 generate → link → simulate → golden compare。完整迁移 `dynamic.exp`、bounds select/update 和 `Gearbox.exp` 的成功仿真 contract，共展开 **62 个 simulation case**：31 Bluesim、31 Icarus。

Windows 下 BSC 生成的 Bluesim 产物是依赖 `sh`/`bluetcl` 的 launcher，Icarus 产物是 `vvp` 字节码而不是 Win32 `.exe`；runner 按 backend 选择启动器，将 `inst/bin/core` 前置到 `PATH`，并为 MSYS `sh` 转换 `BLUESPECDIR`。Icarus 输出按 legacy 规则过滤 `$readmem`、`$finish` 和 `VCD info` 噪声。`CTEST=0` 与 `VTEST=0` 分别显式跳过 Bluesim 和 Verilog/Icarus case。

截至本批，Rust 测试层完整覆盖 **49 个 `.exp` 脚本、212 个独立 contract case**：188 个 upstream 动态 case（126 compile + 62 simulation），加上 24 个 Z3 scheduler case。

## Compile contract 细节

- Frontend argv：`<options> -no-show-timestamps -no-show-version`、可选 `-u`，最后为 `<source>`。
- Verilog argv：`<options> -no-show-timestamps -no-show-version -u -verilog`，module 非空时追加 `-g <module>`，最后为 `<source>`。
- cwd：当前 case 的唯一 workspace。
- 输出：workspace 中的 `<source>.bsc-out`；完整命令、stdout/stderr、退出状态和耗时另写 artifact `bsc.log`。
- Pass：退出成功且 workspace 中存在 `<stem>.bo`；当前 Verilog Pass 也按 upstream `check_intermediate_files` 检查该 `.bo`。
- Fail：退出非零。
- FailWithDiagnostic：在 Fail 基础上，等价统计 Tcl `regexp -all -line {Error:.+\(TAG\)$}` 的行尾 tag；真实 BSC 输出形如 `Error: "file", line ..., column ...: (TAG)`。
- Golden：双方先忽略包含 `SystemC` 或 `dumpfile parameter` 的整行，再按 `diff -b` 语义归一水平空白；不一致时写 `golden.diff`。
- Capability policy：启动时读取 `CTEST`/`VTEST`，默认启用；`CTEST=0` 跳过 `BluesimEnabled`，`VTEST=0` 跳过 `VerilogEnabled`，其他 requirement 仍执行。
- 结果：每个 case 为 Passed、Skipped(reason) 或 Failed(error)，汇总分别计数，只有 failed 影响退出码。
- 隔离：每次 runner 使用 `<pid>-<时间戳>` run-id，每个 case 只清理自己的 workspace/artifact 子目录。
