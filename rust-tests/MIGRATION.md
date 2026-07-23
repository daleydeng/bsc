# Testsuite 全量迁移计划

## 目标与边界

本目录用 Rust 数据模型和可复用 helper 逐项迁移 upstream `testsuite` 的可观察测试契约。迁移对象是 case、fixture、命令参数、退出状态、产物、诊断和 golden 比较规则，**不是实现或嵌入一个 Tcl 解释器**。

Cargo 原生 test harness 继续承载 Rust helper 单测和已经迁移的 scheduler 静态 tests；数量较大、需要运行时发现/筛选的 upstream case 由 `src/bin/upstream.rs` 自定义动态 runner 承载。

只读取 `testsuite/` 中的 fixture 和 golden。Compile case 使用独立 workspace；simulation 由声明式 `SimulationScenario` 生成一次，再把 generation workspace 复制到各 `SimulationContract` 的隔离 workspace。不在原 testsuite 目录中生成或修改文件。

每个 case 模块必须用 `//! Origin:` 注释标出原始 `.exp`。`pixi run just test-alignment` 会把来源脚本中的受支持 Tcl API 调用展开为 compile/Bluesim/Icarus contract multiset，并与 Rust 注册表、golden 声明和 scheduler case 列表逐项比较；默认 `test` 和 `test-upstream` 均将此检查作为前置守门。

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

- 建立 upstream 动态 runner；唯一直接第三方依赖为 `sha2`，仅用于 generation cache 的稳定内容寻址。
- 建立可扩展的 `CompileCase`、`CompileExpectation`、diagnostic 和可选 golden 模型。
- 对齐 `testsuite/config/unix.exp` 的通用 compile 行为：独立 cwd、fixture staging、BSC 参数、`.bo`、非零退出、诊断计数及 golden diff。
- 支持 `--list`、substring filter、`--exact`、`--test-threads N` 和固定 worker queue。
- workspace/artifact 使用进程级唯一 run-id，避免并发 runner 互删。

### Phase 2：机械型 compile 批量迁移（第四批已完成）

- 已逐份核对并迁移 49 个包含 compile contract 的 `.exp` 脚本，按每次公共 compile API 调用展开为 128 个动态 case。
- 覆盖 frontend/Verilog compile mode、compile pass/fail、diagnostic kind/tag/count 与默认 golden；当前 case 全部使用空 options 和 `nodeps=0`。
- Case 数据从公共 runner 中迁出，按 `bluespec_inc` pass、diagnostic fail、golden/mixed 和其他 testsuite 目录拆分。
- 后续机械批次继续启用模型中预留的 `options` 和 `nodeps`，并增加多 fixture、include 路径、额外预期产物等小型通用 contract。

### Phase 3：模板型与后端矩阵（backend capability/policy 已启动）

- 将 Tcl 循环和参数矩阵显式展开为可单独筛选、单独报告的动态 case。
- 已增加一等 Verilog compile mode、Bluesim/Icarus simulation pipeline、`BluesimEnabled`/`VerilogEnabled` requirement、原生 CLI backend policy 和显式 skip 结果；generate、link、simulate 共享进程、日志、超时、归一化和 diff 基础设施。
- 保持每个展开 case 的 fixture/workspace 隔离，避免矩阵并行时共享产物。

### Phase 4：专用脚本

- 对 31 个专用脚本逐一定义 contract，不做 Tcl 语法兼容层。
- 优先提取可复用的文件处理、外部工具调用和输出检查 helper；确实唯一的流程保留小型专用 runner 函数。
- 对平台条件、可选工具和资源需求使用显式 skip/unsupported 结果，不静默降低检查强度。

### Phase 5：全量切换与守护

- 对迁移前后结果做批量双跑，核对 pass/fail、诊断数量、golden 和生成物。
- 持续扩展 alignment parser 和来源元数据，建立覆盖清单与 CI 分片，区分快速 compile、scheduler、backend、simulation 与专用测试。
- 只有在对应 contract 已迁移并双跑稳定后，才考虑从 release 流程移除原 Tcl case。

## Phase 1 已迁移清单

8 个 `.exp` 共展开为 9 个独立动态 case：

| `.exp` | 动态 case | 预期 | 额外检查 |
| --- | --- | --- | --- |
| `b600/b600.exp` | `bsc.bugs/bluespec_inc/b600::Bug600.bsv` | Pass | 生成 `Bug600.bo` |
| `b267/b267.exp` | `bsc.bugs/bluespec_inc/b267::Bug267.bs` | Pass | 生成 `Bug267.bo` |
| `b1040/b1040.exp` | `bsc.bugs/bluespec_inc/b1040::Bug1040.bsv` | Fail | `Error` / `P0127` / 1 |
| `b417/b417.exp` | `bsc.bugs/bluespec_inc/b417::Bug417.bsv` | Fail | `Error` / `S0007` / 1 |
| `b492/b492.exp` | `bsc.bugs/bluespec_inc/b492::Bug492_1.bs` | Fail | `Error` / `T0046` / 1 |
| `b1586/b1586.exp` | `bsc.bugs/bluespec_inc/b1586::Bug1586.bsv` | Fail | `Bug1586.bsv.bsc-out.expected` |
| `b269/b269.exp` | `bsc.bugs/bluespec_inc/b269::Bug269.bsv` | Fail | `Error` / `P0070` / 1；golden |
| `b1493/b1493.exp` | `bsc.bugs/bluespec_inc/b1493::Bug1493.bsv` | Pass | 生成 `Bug1493.bo` |
| `b1493/b1493.exp` | `bsc.bugs/bluespec_inc/b1493::Bug1493_Bad.bsv` | Fail | `Error` / `T0020` / 1 |

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

所有 case 使用包含 testsuite 目录语义的稳定 ID，不保留早期 runner 的短名称别名。每个 case 显式声明 source fixture；9 个 golden case 同时显式声明对应 `<source>.bsc-out.expected` fixture。

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

本批同时引入一等 `CompileMode` 和 `Requirement`：普通 frontend compile 不借用 `options: ["-verilog"]`，Verilog mode 负责 `-u -verilog` 及非空 module 的 `-g`；backend policy 默认启用全部后端，显式禁用 Verilog 时对应 case 明确 SKIP，其他 case 仍运行。结果和汇总区分 passed/skipped/failed，只有 failed 导致非零退出码。

Phase 2 前三批累计迁移 **44 个 `.exp` 脚本、84 个动态 compile case**：36 Pass、11 普通 Fail、37 tagged Fail、13 golden；mode 为 81 frontend、3 Verilog。

### Phase 2 第四批：Verilog 失败 contract

完整迁移 `bsc.evaluator/dynamic/{dynamic.exp,errors/dynamic_errors.exp}` 与 `bsc.arrays/bounds/{select,update}` 中的 42 个 Verilog fail contract：21 个 dynamic error golden、20 个 bounds `S0015` 诊断和 1 个 `DynamicIntegerFail` `T0051` 诊断。至此 compile pipeline 累计 **48 个脚本、126 个 case**：36 Pass、32 普通 Fail、58 tagged Fail、34 golden；mode 为 81 frontend、45 Verilog。所有 compile case 继续使用空 options 和 `nodeps=0`。

### Phase 3 第一批：Bluesim/Icarus simulation matrix

这一阶段最初新增 `SimulationCase`、`SimulationBackend` 与统一 `UpstreamCase` runner，按 backend 独立执行 generate → link → simulate → golden compare。该历史平铺模型现已被声明式 `SimulationScenario` + `SimulationContract` 取代；以下数量仍记录当时迁移批次。完整迁移 `dynamic.exp`、bounds select/update 和 `Gearbox.exp` 的成功仿真 contract，共展开 **62 个 simulation contract**：31 Bluesim、31 Icarus。

Windows 下 BSC 生成的 Bluesim 产物是依赖 `sh`/`bluetcl` 的 launcher，Icarus 产物是 `vvp` 字节码而不是 Win32 `.exe`；runner 按 backend 选择启动器，将 `inst/bin/core` 前置到 `PATH`，并为 MSYS `sh` 转换 `BLUESPECDIR`。Icarus 输出过滤 `$readmem`、`$finish` 和 `VCD info` 非确定性噪声；`--no-bluesim` 与 `--no-verilog` 分别显式跳过对应 backend case。

截至第一批，Rust 测试层完整覆盖 **49 个 `.exp` 脚本、212 个独立 contract case**：188 个 upstream 动态 case（126 compile + 62 simulation），加上 24 个 Z3 scheduler case。

### Phase 3 第二批：scheduler conflict-free

完整迁移 `bsc.scheduler/conflict_free/conflict_free.exp`，新增 20 个独立 contract：18 个 simulation case（9 Bluesim + 9 Icarus）、1 个 Verilog `G0002` diagnostic fail 和 1 个 Verilog `G0010` warning pass。Simulation 覆盖 backend-specific expected，并验证 `ConflictFreeOK3` 的 `-aggressive-conditions` generate option。

本批增加 `PassWithDiagnostic`，在编译成功和 `.bo` 产物检查基础上精确统计指定 warning；simulation 声明也开始支持非空 compile options。当前累计完整覆盖 **50 个 `.exp` 脚本、232 个独立 contract case**：208 个 upstream 动态 case（128 compile + 80 simulation），加上 24 个 Z3 scheduler case。

### Phase 3 第三批：dynamic strings

完整迁移 `bsc.evaluator/dynamic/strings/dynamic_strings.exp`，新增 14 个 simulation contract（7 Bluesim + 7 Icarus）。按原 Tcl exclusion 建模 Icarus 版本能力：`StringInteger` 要求 Icarus >= 12，`StringIntegerWithNull` 要求 Icarus >= 13；runner 从 `iverilog -V` 探测主版本，低版本 case 保持注册并显式 Skipped。

当前累计完整覆盖 **51 个 `.exp` 脚本、246 个独立 contract case**：222 个 upstream 动态 case（128 compile + 94 simulation），加上 24 个 Z3 scheduler case。

### Phase 3 第四批：bound type variables 与 bug 810

完整迁移 `bsc.typechecker/kind/bound-vars/bound-vars.exp` 的 8 个 frontend compile contract（2 pass + 6 tagged fail），以及 `bsc.bugs/bluespec_inc/b810/b810.exp` 的 1 个 tagged compile fail 和 6 个 simulation contract（3 Bluesim + 3 Icarus）。两组均无额外 options、exclusions 或 compile golden。

当前累计完整覆盖 **53 个 `.exp` 脚本、261 个独立 contract case**：237 个 upstream 动态 case（137 compile + 100 simulation），加上 24 个 Z3 scheduler case。

### Phase 3 第五批：read desugaring、case syntax 与 bug 235

完整迁移 `bsc.typechecker/read_desugaring/read_desugaring.exp`、`bsc.syntax/bsv05/case/case.exp` 和 `bsc.bugs/bluespec_inc/b235/b235.exp`，新增 16 个 frontend compile contract 与 12 个 simulation contract。该批覆盖普通 fail、tagged fail、pass、双 backend simulation，以及 3 个 frontend `.bsc-out.expected` golden。

当前累计完整覆盖 **56 个 `.exp` 脚本、289 个独立 contract case**：265 个 upstream 动态 case（153 compile + 112 simulation），加上 24 个 Z3 scheduler case。

### Phase 3 第六批：小型 regression scripts

完整迁移 8 个单调用或低风险脚本：`b1048`、`b1163`、`b1198`、`b1229`、`b1318`、`b1037`、`b1045` 和 `gh894`。新增 6 个 compile contract 与 4 个 simulation contract；`gh894` 保持 upstream 的普通 frontend fail + golden 语义，不因输出中恰好包含诊断 tag 而收紧 contract。

当前累计完整覆盖 **64 个 `.exp` 脚本、299 个独立 contract case**：275 个 upstream 动态 case（159 compile + 116 simulation），加上 24 个 Z3 scheduler case。

### Phase 3 第七批：直接 compile/simulation 脚本

完整迁移 6 个无需扩展 runner 的脚本：`b120`、`EAmbOper`、`properties`、`FIRFilter`、`Hamming` 和 `BRAMTest`。新增 6 个 compile contract 与 12 个 simulation contract；FIR 显式 stage 其本地 package source，BRAM 显式 stage 运行时初始化文件 `bram2.txt`。

当前累计完整覆盖 **70 个普通 `.exp` 脚本、317 个独立 contract case**：293 个 upstream 动态 case（165 compile + 128 simulation），加上 24 个 Z3 scheduler case。

### Phase 3 第八批：简单 compile regressions

完整迁移 12 个单调用 compile 脚本：`b1043`、`b1213`、`b1235`、`b1265`、`b1267`、`b1332`、`b1356`、`b1389`、`b1396`、`b265`、`b290` 和 `b308`。新增 5 个 frontend pass、6 个 Verilog pass 和 1 个带 `G0124` 诊断的 Verilog fail；有本地 package 依赖的 case 显式 stage 全部源文件。

当前累计完整覆盖 **82 个普通 `.exp` 脚本、329 个独立 contract case**：305 个 upstream 动态 case（177 compile + 128 simulation），加上 24 个 Z3 scheduler case。

### Phase 3 第九批：跨目录简单 compile regressions

完整迁移 15 个单调用 compile 脚本，覆盖 `bluespec_inc`、GitHub regressions、BSV examples 和 evaluator static-eval。新增 7 个 frontend pass、7 个 Verilog pass 和 1 个带 `S0015` 诊断的 Verilog fail；`b373` 显式 stage 本地依赖 `Wallace.bs`，其余 case 均为单 source fixture。

当前累计完整覆盖 **97 个普通 `.exp` 脚本、344 个独立 contract case**：320 个 upstream 动态 case（192 compile + 128 simulation），加上 24 个 Z3 scheduler case。

### Phase 3 第十批：大批量静态 compile scripts

完整迁移 60 个无需扩展 runner 的静态 compile 脚本：30 个来自 `bluespec_inc`，30 个来自 messages、interra、port renaming、GitHub regressions、BSV examples 和 typechecker。新增 93 个 compile contract，包括 47 个 pass、18 个普通 fail 和 28 个 tagged fail；其中 66 个 frontend、27 个 Verilog、25 个 golden、8 个非空 options、4 个显式 module。本地 package 的递归依赖均显式 stage，静态多调用 `.exp` 的每个 contract 均独立注册。

当前累计完整覆盖 **157 个普通 `.exp` 脚本、437 个独立 contract case**：413 个 upstream 动态 case（285 compile + 128 simulation），加上 24 个 Z3 scheduler case。

### Phase 3 第十一批：多调用 compile 与 contract inventory

完整迁移 31 个静态 compile 脚本：6 个来自 `bluespec_inc`，25 个来自 interra bugs/messages 和 BSV examples。新增 64 个 compile contract，包括 26 个 pass、25 个普通 fail、12 个 tagged fail 和 1 个 tagged warning；其中 39 个新增 golden、4 个非空 options、3 个显式 module，并覆盖诊断 count=2 和大型递归 fixture 闭包。

`alignment` 同时新增全仓库 contract inventory：除脚本覆盖率外，统计当前模型可静态识别的 compile、Bluesim、Icarus 和 scheduler contract，并明确报告仍需动态或自定义 Tcl 分析的脚本数。该 contract 分母是可重复验证的静态声明数，不假装展开 Tcl 循环或自定义流程。

当前累计完整覆盖 **188 个普通 `.exp` 脚本、501 个独立 contract case**：477 个 upstream 动态 case（349 compile + 128 simulation），加上 24 个 Z3 scheduler case。计入 `sat.exp` 后，脚本覆盖为 **189/860**，剩余 671 个；静态 contract 覆盖为 **501/4161**，剩余 3660 个；另有 250 个脚本需要动态或自定义 Tcl 分析。

### Phase 3 第十二批：静态多调用 compile 与 simulation regressions

完整迁移 50 个静态脚本、117 个 contract：30 个 compile 脚本展开为 73 个 case，20 个 simulation 脚本展开为 44 个 backend case。Compile 新增 17 个普通 pass、4 个 tagged warning pass、19 个普通 fail 和 33 个 tagged fail，其中 55 个 frontend、18 个 Verilog、25 个 golden、17 个非空 options；simulation 新增 22 个 Bluesim 和 22 个 Icarus contract，并递归 stage `b1302`、`stepcounter`、`xbar` 的本地依赖。

当前累计完整覆盖 **238 个普通 `.exp` 脚本、618 个独立 contract case**：594 个 upstream 动态 case（422 compile + 172 simulation），加上 24 个 Z3 scheduler case。计入 `sat.exp` 后，脚本覆盖为 **239/860**，剩余 621 个；静态 contract 覆盖为 **618/4161**，剩余 3543 个；另有 250 个脚本需要动态或自定义 Tcl 分析。

Rust case 模块采用稳定的“来源范围 + contract 形态”命名，例如 `bluespec_inc_single`、`bluespec_inc_multi`、`bluespec_inc_golden`、`cross_suite_basic`、`cross_suite_direct`、`cross_suite_errors`、`cross_suite_golden`、`cross_suite_mixed`、`cross_suite_multi` 和 `static_regressions`。迁移批次编号只保留在本文件的时间线中，不进入 Rust module 名；case 模块文件名不使用 `batch`、`large`、`other` 或批次序号，避免后续迁移改变代码结构语义。

注册架构随后完成去中心化：每个来源模块维护自己的 `CASES` slice，compile/simulation 中央文件分别只保留一个按稳定名称排序的模块宏列表，由同一列表同时生成 `mod` 声明和模块描述，再使用标准库 `OnceLock` 一次展平。原先中央 422 项和 172 项手工数组已删除，数据模型单测也不再保存会随迁移频繁变化的总数、类别和 backend 快照，而是验证逐 case 语义关系、非空集合和全局名称唯一性。

`alignment` 对模块架构执行闭环检查：磁盘 `.rs` 文件必须与宏注册集合一致，模块不得为空或使用迁移过程命名；文件头必须显式列出完整 `Origin(s)`，且与模块 `CASES` 根据 `fixture_dir` 推导出的 `.exp` 集合完全一致。原有 contract multiplicity、golden、fixture 和 scheduler 对齐检查继续保留，因此遗漏整个模块、遗漏模块内 case、写错来源或留下孤立文件都会在运行 BSC 前失败。重构后的完整 `pixi run just test` 验证为 30 个 helper、24 个 scheduler 和 594 个 upstream case 全部通过，422/422 BSC result 与 172/172 generation cache 命中。

剩余工作也已固化为可检查产物：`remaining` Rust binary 复用 alignment 的来源注册与静态 contract 解析，生成 [`REMAINING.md`](REMAINING.md) 中全部未迁移 `.exp`，并验证脚本数与 contract 总和严格等于 alignment summary；[`NEXT.md`](NEXT.md) 保存人工核对过的安全候选、迁移顺序和阻塞原因。`inventory-update` 用于迁移后重建清单，`inventory-check` 已进入默认测试前置守门，避免文档再次过期。

### Phase 3 第十三批：通用 artifact assertions 与 classic simulation

Compile contract 新增一等 `ArtifactAssertion` / `TextAssertion`，支持文件存在性、固定字符串存在/不存在、固定字符串匹配行数、多行正则及诊断 tag 精确计数。检查在 compile/golden 后执行；BSC result cache 命中恢复 workspace 后仍重新执行 assertions，不把最终 pass/fail 缓存起来。

Alignment 同步解析 `find_n_strings`、`string_occurs`、`string_does_not_occur`、`find_regexp`、`find_n_regexp` 和 `find_n_emsg`，包括 Tcl 行续写、brace/quoted 参数及 `[make_bsc_output_name ...]` 路径，并与 Rust assertion 逐项、逐数量核对。classic `.bs` helper `test_c_veri`、`test_c_veri_bs_modules`、`test_c_veri_bs_modules_options` 归一化为 shared elaboration 下的 Bluesim/Icarus 双 contract；补齐其静态权重后，可静态识别的全仓库 contract 分母由旧估算 4368 校正为 4600。

本批完整迁移 5 个来源、149 个 contract：`bsc.typechecker/mismatch` 12 个 compile、`enableReady` 6 个 compile、`moduleArgs` 39 个 compile、`context-errors` 59 个 compile，以及 `constructors` 的 29 个 compile 和 4 个双后端 simulation contract。当前累计覆盖 **315/860** 个来源和 **1454/4600** 个静态 contract；剩余 **545** 个来源、**3146** 个静态 contract，另有 226 个脚本需要动态或自定义 Tcl 分析。

### 测试执行架构收敛

删除早期按单 contract 打平的 `UpstreamCase`、`all_cases`、`select_cases` 和 `build_work_items` 兼容层。CLI 现在直接从 compile registry 与 scenario registry 构造 `ExecutionPlan`；simulation contract 在选择阶段始终保留所属 `SimulationScenario`，runner 不再通过指针扫描猜测并重建分组。早期 `bluespec_inc` 短 case ID 也全部替换为与其他模块一致的来源路径式稳定 ID，不提供旧名称别名。

Backend policy 同步脱离 Tcl harness 的 `CTEST`/`VTEST` 环境变量，改用原生 `--no-bluesim` / `--no-verilog` CLI。代码中的 `legacy_*` golden 命名已收敛为实现语义命名；golden 归一化和 Icarus 噪声过滤本身仍作为 upstream contract 的必要行为保留。

### 统一 artifact comparison 与 schedule contract

Compile 与 simulation pipeline 现在共用 `upstream/artifact.rs` 的 `ArtifactAssertion` 执行器。除文件存在和文本断言外，`Matches` 支持 `Exact`、`GoldenOutput`、`Verilog` 三种归一化策略：`Exact` 做字节精确比较；`GoldenOutput` 对齐 upstream 的换行、scientific exponent、`diff -b` 空白和已知噪声规则；`Verilog` 进一步移除 compiler banner，并归一化 generated identifier 中的数字后缀。不一致时统一生成 `artifact-N.diff`。

Artifact actual 必须是 workspace 内的安全相对路径；expected 必须作为 fixture 显式声明，且不得与 actual 指向同一文件。BSC result cache 或 generation cache 命中只恢复 workspace，所有 compile/simulation assertions 仍会重新执行；simulation 的普通输出、VCD contract 和 backend-specific side effect 也使用同一模型。

新增 `CompileMode::VerilogSchedule`，严格对齐 `compile_verilog_schedule_pass` 的 `-resource-simple -show-schedule -dschedule -dresources -dvschedinfo -verilog` 参数。Alignment 同时解析并逐数量核对 `compare_file` 与 `compare_verilog`；识别 schedule helper 后，全仓库可静态识别 contract 分母由 4600 校正为 4672，其中新增 72 个是此前未识别的 upstream schedule contract。

本批完整迁移 4 个来源、52 个 contract：`error_recovery` 11 个 compile，`CompletionBuffer` 1 个 schedule compile和 2 个 simulation，`Cntrs` 2 个 compile 和 6 个 simulation，`fwrite` 7 个 compile fail 和 23 个 backend-specific simulation。随后迁移 `SShow` 的 1 个指定 top Verilog compile contract，并通过同一 artifact runner 比较 elaboration 生成的 `sysTestSShow.out`；完整迁移 `paths` 的 56 个 Verilog compile contract、56 条正向和 77 条负向 generated RTL regex assertion。`TextAssertion::RegexDoesNotMatch` 与 alignment 的 `find_regexp_fail` 成为一等 contract；文本断言在入口按 Tcl 文本读取语义统一 CRLF/CR，alignment 将 Tcl Verilog helper 的 `.bsc-vcomp-out` 映射为 runner 的 canonical `.bsc-out`，不在 runner 中生成兼容 alias。

随后完整迁移 `import-foreign` 的 74 个 compile contract，覆盖 warning count、compiler golden、generated RTL regex 和 2 个 `compare_verilog`。新增 Rust-only `ArtifactAssertion::ParsesAsSystemVerilog`，使用 `sv-parser 0.13.5` 对现有 4 个 Verilog golden actual 做 IEEE 1800-2017 syntax smoke；parser assertion 不进入 Tcl multiplicity，且不替代 normalized golden，真实 Verilog 工具链兼容继续由 Icarus 验证。当前累计覆盖 **322/860** 个来源和 **1637/4672** 个静态 contract；剩余 **538** 个来源、**3035** 个静态 contract，另有 221 个脚本需要动态或自定义 Tcl 分析。

### Migration readiness 与首轮自动候选迁移

剩余 inventory 进一步分析每份未迁移 `.exp` 的活动 Tcl command vocabulary，按 `candidate`、`review`、`blocked`、`dynamic/custom` 分类，并汇总 unsupported command 的类别、调用次数、影响脚本与静态 contract。Curated blocker registry 与未迁移来源集合双向守门；blocker 已迁移、删除或路径漂移时 `inventory-check` 会立即失败。静态 contract 分母由单元测试固定为 4672，避免 readiness 分析改变覆盖口径。

首轮 analyzer 得到 33 个 lexical candidate、121 个静态 contract。全部候选经逐份 fixture/options/golden/bug-gate review 后整份迁移：新增 32 个 compile contract，以及 51 个 simulation scenario 展开的 89 个 backend contract，覆盖 vector、string/generics、BH pragma、library runtime、Verilog golden、warning count、额外 generated module、backend-specific generation、空 golden、VCD 和 artifact comparison。迁移后 lexical candidate 队列归零，累计覆盖 **355/860** 个来源和 **1758/4672** 个静态 contract；剩余 **505** 个来源、**2914** 个静态 contract，仍有 221 个脚本需要动态或自定义 Tcl 分析。

首次完整运行通过 52 个 helper tests、24 个 scheduler tests 和 1734 个 upstream dynamic contracts；新增批次产生 32 个 BSC result cache miss/store 与 51 个 generation cache miss/store，全部 contract 通过。

### Generation cache 与性能基线

默认 `test` 对成功的 simulation generation workspace 使用 SHA-256 内容寻址缓存；cache hit 仍重新执行 link、simulation 与 golden compare。完整 cache-fill 冷运行的 upstream artifact wall time 为 **435.5 秒**，128 个 simulation generation 全部 miss 并写入；随后完整热运行 128/128 hit，artifact wall time 为 **17.4 秒**，293 个 upstream case 均通过。

Bluesim link 的生成 C++ 编译进一步使用 Pixi 管理的 `ccache`。在相同 128/128 generation hit 条件下，A/B 实测 `ccache 4.13.6` 和 `sccache 0.16.0` 的 cacheable warm hit 均为 128/128；`ccache` 的完整 upstream wall time 为 **15.35 秒**、Bluesim link 累计 **44.8 秒**，优于 `sccache` 的 **17.69 秒**和 **67.0 秒**，因此 Windows 默认选用 `ccache`。

当前 422 个 compile contract 与 24 个 scheduler contract 使用统一 BSC result cache。缓存 key 包含 toolchain、fixture、argv、关键环境及一次性计算的 Z3 内容指纹；只有已通过对应 Rust contract 和 golden 的原始 BSC workspace、输出和 exit status 才会原子发布，cache hit 后仍重新执行全部 Rust 检查。第七批完成时 warm 全量 `pixi run just test` 实测 **13.59 秒**：24/24 scheduler result hit、165/165 compile result hit、128/128 generation hit，27 个 helper、24 个 scheduler 和 293 个 upstream case 全部通过；第十一批迁移后再次 warm 验证 349/349 compile、128/128 generation cache hit，28 个 helper 和 477 个 upstream case 全部通过；第十二批首次全量验证得到 349 个 compile hit、73 个 miss/store、128 个 generation hit 和 44 个 miss/store，随后 warm 验证为 422/422 compile 与 172/172 generation hit，28 个 helper、24 个 scheduler 和 594 个 upstream case 全部通过。

`pixi run just test-cold` 会同时禁用 generation cache、BSC result cache 和 compiler cache，保留完整无缓存验证入口。

## Compile contract 细节

- Frontend argv：`<options> -no-show-timestamps -no-show-version`、可选 `-u`，最后为 `<source>`。
- Verilog argv：`<options> -no-show-timestamps -no-show-version -u -verilog`，module 非空时追加 `-g <module>`，最后为 `<source>`。
- VerilogSchedule argv：在 Verilog 模式基础上加入 `-resource-simple -show-schedule -dschedule -dresources -dvschedinfo`，严格对齐 upstream schedule helper。
- cwd：当前 case 的唯一 workspace。
- 输出：workspace 中的 `<source>.bsc-out`；完整命令、stdout/stderr、退出状态和耗时另写 artifact `bsc.log`。
- Pass：退出成功且 workspace 中存在 `<stem>.bo`；当前 Verilog Pass 也按 upstream `check_intermediate_files` 检查该 `.bo`。
- PassWithDiagnostic：在 Pass 基础上，精确统计指定 kind/tag/count，用于 `compile_verilog_pass_warning` 等 contract。
- Fail：退出非零。
- FailWithDiagnostic：在 Fail 基础上，等价统计 Tcl `regexp -all -line {Error:.+\(TAG\)$}` 的行尾 tag；真实 BSC 输出形如 `Error: "file", line ..., column ...: (TAG)`。
- Golden：双方先忽略包含 `SystemC` 或 `dumpfile parameter` 的整行，再按 `diff -b` 语义归一水平空白；不一致时写 `golden.diff`。
- Artifact assertions：compile 与 simulation 共用同一执行器，在 cache 恢复后的 workspace 中检查安全相对路径；`Contains`/`DoesNotContain` 检查固定字符串，`LineCount` 统计包含固定字符串的行数，`Regex`/`RegexCount` 使用 multiline Rust regex，`DiagnosticCount` 复用统一 diagnostic 计数。`Matches` 支持 `Exact`、`GoldenOutput`、`Verilog` 三种比较；expected 必须是显式 fixture，且不得与 actual 相同；不一致时写 `artifact-N.diff`。
- Capability policy：默认启用全部后端；`--no-bluesim` 跳过 `BluesimEnabled`，`--no-verilog` 跳过所有 Verilog/Icarus requirement；`IcarusAtLeast(N)` 通过 `iverilog -V` 探测主版本，版本不足或无法确定时显式跳过。
- 结果：每个 case 为 Passed、Skipped(reason) 或 Failed(error)，汇总分别计数，只有 failed 影响退出码。
- 隔离：每次 runner 使用 `<pid>-<时间戳>` run-id，每个 case 只清理自己的 workspace/artifact 子目录。
