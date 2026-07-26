# Rust testsuite migration plan

本文件记录已经人工核对过的迁移候选、实施顺序和阻塞原因。完整且自动更新的剩余 `.exp` 清单见 [`REMAINING.md`](REMAINING.md)。

## 当前基线

- 已迁移来源：511/860
- 尚未迁移来源：349
- 已迁移 typed contract：2407/5533
- 尚未迁移 typed contract：3126
- 没有 typed contract 的脚本：137
- 当前 inventory 完全由 typed manifest 生成，不再保留手写 Tcl parser 或词法计数分母

候选进入本文件的硬条件：必须完整迁移一整份 `.exp` 的全部活动 contract 和 assertion；不得只摘取 compile/simulation 调用；不得忽略 XFAIL、bug gate、generated artifact、手工 link/sim 或额外输出比较。

## 自动候选队列

[`REMAINING.md`](REMAINING.md) 由 `pixi run just inventory-update` 从 typed manifest、Rust registry 和 curated blocker registry 同源生成，是当前剩余范围与迁移 readiness 的唯一事实来源。当前 typed candidate 队列已经清空。Manifest schema v4 已将 `compile_object_pass`、`link_objects_pass`、`sim_output`、`copy` 和 `move` 降低为带 guard/span/expansion 的 typed workflow action，并按 producer/consumer guard coverage、top-level executable、link segment 和 stdout artifact flow 保守组合为 139 个 Bluesim workflow、152 个有效 run-or-link contract。原 1027 个 action 中仍有 595 个歧义 action 或 side-artifact action 留在 85 个脚本中等待 review；101 个静态 `sim_output` 中已有 98 个完成 link 关联。Rust runner 现已提供独立 `BluesimWorkflowScenario` 执行内核、持久化 build cache 和逐字段 manifest alignment，当前 17 个真实来源、37 个 workflow contract 已覆盖单 run、link-only、多 generation/多 run/transfer，以及 Library latency 的批量单 generation/link/run/golden 形态。下一步继续批量迁移已组合且无额外 side action 的 workflow。

`candidate` 表示该 `.exp` 的活动 Tcl command vocabulary 已被现有 Rust contract/assertion 模型覆盖，且不在已知 blocker registry 中；它仍不是自动批准。批量迁移时必须逐份完整 review fixture、options、golden、条件分支、bug gate 与运行结果。`inventory-check` 会守护生成文档、candidate 分类以及 blocker registry 是否与当前未迁移集合一致。

## 已完成：Bluesim workflow 执行内核

新增独立于跨后端 `SimulationScenario` 的 `BluesimWorkflowScenario`，原生表达多 generation、link-only、顺序多 run、stdout artifact copy/move 与 assertions。runner 严格复刻 upstream helper 参数顺序，将 generation/link 产物作为一个持久化 build-cache snapshot；cache 命中后仍重新执行 run 与所有 assertions。Alignment 使用 frontend 的非执行静态 Tcl-list parser，把 manifest options/object list 转为 argv，并对 generation、link、run 和 transfer canonical signature 逐字段核对。

首批三个真实来源、四个 workflow contract 已整体迁移并实际通过：`b1489.exp` 覆盖单 generation/run 与文本断言，`b1243.exp` 覆盖 link-only，`traffic_light_controller_separate.exp` 覆盖双 generation、顺序双 run、stdout copy 与两个 golden。第二批整体迁移 `bsc.interra/Library_latency` 下 7 个来源和 `bsc.lib/sram/sram.exp`，新增 24 个单 generation/link/run/golden workflow contract；其中 SRAM/SyncRAM 所需 `Precedence.bs` 均显式 stage。第三批迁移 `debugging.exp`、`b1439.exp` 和 `b1796.exp`，新增 6 个 build-only workflow contract，覆盖递归本地 fixture、同 top 的 `.bs`/`.bsv` 独立流程，以及无 module/空 object link。第四批完整迁移 `eq3.exp` 和 `parse_strings.exp` 的 10 个 mixed contract，其中包括 2 个 build-only workflow、6 个 frontend/Verilog compile 和 2 个双后端 simulation contract。第五批完整迁移 `rdy_en_pragmas.exp` 的 23 个 mixed contract，包括 14 个 Verilog compile、8 个双后端 simulation 和 1 个 build-only workflow，并把共享 generation warning assertion 绑定到实际生产日志的 Icarus contract。五批均完成 Windows 实际运行和 cache-hit 复跑。Workflow contract 现正式进入 typed contract 总分母。

## 已完成：第二轮 typed candidate 队列

完整迁移 6 个来源、39 个 typed contract：`derived_bits` 17 个 generated-Verilog compile contract，`gh276` 6 个 compiler golden，Gating `attributes` 6 个 Verilog port/diagnostic contract，`b752` 2 个 diagnostic artifact contract，以及 Divide、SquareRoot 各 4 个 backend-specific simulation contract。所有模块均保留精确 `Origin`，并通过 typed alignment、fixture/options/golden/assertion review 和实际运行。

Tree-sitter frontend 同时修正未加引号 Tcl composite word 的静态降低：`${name}.bs` 和 `mk${name}Reg.v` 现在按相邻 CST fragment 合并为单个 Tcl word，并有独立回归测试。SquareRoot 的 Bluesim 随机样本来自平台 RNG：Windows `rand32` 使用 C `rand()` 组合，无法与 POSIX `random()` 生成的 upstream golden 逐值一致；`OutputNormalization::MaskedLines` 因此只遮蔽 `sqrt (` 随机样本行内容，仍严格核对固定测试向量、章节顺序和样本数量。Icarus 继续使用完整 golden 比较。

本批实际运行结果为 39/39 通过；SquareRoot 首次 generation 后再次运行得到 4/4 generation cache hit。

## 已完成：首轮自动候选队列

首轮 analyzer 识别出的 33 个候选已全部逐份 review 并整份迁移，共新增 121 个 contract：32 个 compile contract，以及 51 个 simulation scenario 展开的 89 个 backend contract。覆盖 vector/string/generics/BH pragma、library runtime、Verilog golden、backend-specific simulation、VCD、额外 fixture、warning count 和 artifact comparison；首次完整运行 `1734/1734` 通过，并写入 32 个 BSC result cache 与 51 个 generation cache。

## 已完成：纯静态 compile 第一批

以下 13 个脚本无需扩展 runner、无 compiler golden、无额外本地 fixture、无未迁移活动 assertion，已迁移为 `cross_suite_frontend_static` 和 `cross_suite_verilog_static` 两个稳定语义模块，共新增 157 个 compile contract。

### Frontend：4 个脚本，44 个 contract

- [x] `testsuite/bsc.syntax/bh/underscore/underscore.exp` — 23：Pass×12，tagged Fail×11
- [x] `testsuite/bsc.syntax/bsv05/attribs/attribs.exp` — 7：tagged Fail×7
- [x] `testsuite/bsc.bsc_examples/trafficlight/trafficlight.exp` — 10：Pass×10
- [x] `testsuite/bsc.typechecker/ctxreduce/ctxreduce.exp` — 4：Pass×4

### Verilog/codegen：9 个脚本，113 个 contract

- [x] `testsuite/bsc.mcd/Pragmas/Pragmas.exp` — 28：Verilog Pass×9，tagged Verilog Fail×19
- [x] `testsuite/bsc.interra/Path_Analysis/Single_Module/Single_Module.exp` — 21 tagged Verilog Fail；3 个 count=2；`MuxLogic.bsv` 有两个不同 tag contract
- [x] `testsuite/bsc.interra/Path_Analysis/Extended_Input_Output_Path/Extended_Input_Output_Path.exp` — 9 tagged Verilog Fail；`Argument2Rdy.bsv` count=2
- [x] `testsuite/bsc.names/portRenaming/conflicts/modarg/modarg.exp` — 18：Verilog Pass×4，tagged Verilog Fail×14
- [x] `testsuite/bsc.names/portRenaming/conflicts/modparam/modparam.exp` — 18：Verilog Pass×4，tagged Verilog Fail×14
- [x] `testsuite/bsc.names/portRenaming/invalidAttrs/port/port.exp` — 7 tagged Verilog Fail
- [x] `testsuite/bsc.names/portRenaming/invalidAttrs/enable/enable.exp` — 4 tagged Verilog Fail
- [x] `testsuite/bsc.names/portRenaming/invalidAttrs/ready/ready.exp` — 4 tagged Verilog Fail
- [x] `testsuite/bsc.names/portRenaming/invalidAttrs/result/result.exp` — 4 tagged Verilog Fail

## 已完成：其余可运行的静态 compile 候选

此前严格核对的 35 个静态 compile 脚本中已完成 34 个：第一批 13 个、157 个 contract，本批 21 个、300 个 contract。`performance.exp` 在原生 Windows 上因 `BNotShared.bsv` 超过 300 秒仍未完成，已转入阻塞清单。

### Compiler golden 密集型：12 个，201 个 contract，126 个 golden

- [x] `testsuite/bsc.typechecker/kind/kind.exp` — 44；37 golden
- [x] `testsuite/bsc.interra/preprocessorTestcases/define/define.exp` — 33；33 golden
- [x] `testsuite/bsc.typechecker/primtcons/primtcons.exp` — 27；6 golden；含 Verilog；一个 error count=2
- [x] `testsuite/bsc.bugs/github/gh353/gh353.exp` — 18；9 golden；递归 stage `Bug353_Type.bs`、`Types.bs`、`Types_NonNamed.bs`
- [x] `testsuite/bsc.typechecker/fundeps/fundeps.exp` — 17；2 golden；`StructUpdateOneDimArray.bs` 使用 `-let-gen`；一个 error count=2
- [x] `testsuite/bsc.typechecker/foreignmodule/parameters/parameters.exp` — 12；2 golden
- [x] `testsuite/bsc.typechecker/foreignmodule/ports/ports.exp` — 12；3 golden
- [x] `testsuite/bsc.typechecker/kind/mismatch/mismatch.exp` — 11 tagged Fail；11 golden
- [x] `testsuite/bsc.bugs/github/gh221/gh221.exp` — 9；5 golden
- [x] `testsuite/bsc.interra/preprocessorTestcases/ifdef/ifdef.exp` — 6；6 golden
- [x] `testsuite/bsc.interra/preprocessorTestcases/undef/undef.exp` — 6；6 golden
- [x] `testsuite/bsc.syntax/bsv05/dups/dups.exp` — 6 Fail；6 golden

### 其他静态 frontend：5 个，60 个 contract

- [x] `testsuite/bsc.syntax/bsv05/interface/interface.exp` — 28；4 个 upstream `compile_fail source TAG` 调用的 TAG 保留在 options 参数
- [x] `testsuite/bsc.typechecker/assignment/assignment.exp` — 18；`StructUpdReg.bsv` 保留 upstream options；`RegStructWrite.bsv` 保留 frontend 与 Verilog 两个 contract
- [x] `testsuite/bsc.interra/messages/EBadIfcType/EBadIfcType.exp` — 5；frontend tagged Fail，options 含 `-verilog -g <top>`，不是 Verilog mode
- [x] `testsuite/bsc.preprocessor/ifdef/ifdef.exp` — 5 Pass
- [x] `testsuite/bsc.driver/symtab/symtab.exp` — 4 Pass；递归 stage 8 个 `*_Wrapper.bsv` / `*_Leaf.bsv`

### 其他静态 Verilog/codegen：4 个已完成，39 个 contract

- [x] `testsuite/bsc.names/portRenaming/conflicts/miscellaneous/conflicts.exp` — 21 frontend tagged Fail，options=`-verilog`；count 最大 7
- [x] `testsuite/bsc.bsv_examples/fsm/fsm.exp` — 9；frontend+Verilog；显式 module=`mkFSM`
- [x] `testsuite/bsc.bugs/bluespec_inc/b1490/b1490.exp` — 6 Verilog Pass；固定 RTS options=`+RTS -M288M -Sstderr -RTS`
- [ ] `testsuite/bsc.evaluator/performance/performance.exp` — 3 Verilog Pass；`BNotShared.bsv` 在原生 Windows 上超过 300 秒，暂不迁移
- [x] `testsuite/bsc.names/portRenaming/conflicts/readyResult/readyResult.exp` — 3 frontend tagged Fail，options=`-verilog`；`Test01.bsv` 两个相同调用使用唯一 Rust name，multiplicity=2

## 已核对的静态 simulation 候选

当前确认的 39 个静态 simulation 脚本已完成 37 个；`FloatTest.exp` 和 `BRAM0Test.exp` 因原生 Windows 共享 elaboration 超时而整体回退为性能 blocker。带 compile contract 的 origin 已同步迁移 compile case；alignment 会拒绝半迁移。复杂 case 同时覆盖多生成模块、递归 fixture、case-local 编译/链接参数和标准 VCD 行为。

### 已完成：简单静态 23 个

- [x] `testsuite/bsc.verilog/schedule/schedule.exp`
- [x] `testsuite/bsc.verilog/tasks/real/real_tasks.exp`
- [x] `testsuite/bsc.verilog/tasks/time/time.exp`
- [x] `testsuite/bsc.typechecker/display/display.exp`
- [x] `testsuite/bsc.syntax/bsv05/stmt/stmt.exp`
- [x] `testsuite/bsc.misc/bitextract/bitextract.exp`
- [x] `testsuite/bsc.misc/format/format.exp`
- [x] `testsuite/bsc.mcd/ClockMux/clockmux.exp`
- [x] `testsuite/bsc.mcd/Synchronizers/synchronizers.exp`
- [x] `testsuite/bsc.mcd/SyncReset/SyncReset.exp`
- [x] `testsuite/bsc.lib/BuildVector/BuildVector.exp`
- [x] `testsuite/bsc.lib/dreg/dreg.exp`
- [x] `testsuite/bsc.lib/Memory/Memory.exp`
- [x] `testsuite/bsc.lib/Printf/Printf.exp`
- [x] `testsuite/bsc.lib/TreeMap/libtreemap.exp`
- [x] `testsuite/bsc.lib/BRAM/SyncBRAMFIFO/SyncBRAMFIFO.exp`
- [x] `testsuite/bsc.evaluator/prims/when/when.exp`
- [x] `testsuite/bsc.bugs/bluespec_inc/b1424/b1424.exp`
- [x] `testsuite/bsc.bugs/bluespec_inc/b1658/b1658.exp`
- [x] `testsuite/bsc.bugs/bluespec_inc/b540/b540.exp`
- [x] `testsuite/bsc.bsv_examples/SHA1/SHA1.exp`
- [x] `testsuite/bsc.bsv_examples/SHA256/SHA2.exp`
- [x] `testsuite/bsc.bsv_examples/SHA512/SHA2.exp`

### Options 或递归 fixture：14 个

- [x] `testsuite/bsc.mcd/MakeClock/MakeClock.exp` — compile options `-keep-fires`
- [ ] `testsuite/bsc.lib/FloatingPoint/FloatTest.exp` — 共享 `-verilog -elab` 在原生 Windows 串行运行超过 600 秒；整份脚本暂不迁移
- [x] `testsuite/bsc.lib/PAClib/RadixSort/rev1/paclib_radix_rev1.exp` — `RadixSort.bsv`、`Types.bsv`；Bluesim VCD 输出一致性
- [x] `testsuite/bsc.lib/PAClib/RadixSort/rev2/paclib_radix_rev2.exp` — `RadixSort.bsv`、`Types.bsv`；Bluesim VCD 输出一致性
- [x] `testsuite/bsc.lib/PAClib/RadixSort/rev3/paclib_radix_rev3.exp` — `RadixSort.bsv`、`Types.bsv`；Bluesim VCD 输出一致性
- [x] `testsuite/bsc.lib/PAClib/RadixSort/rev4/paclib_radix_rev4.exp` — `RadixSort.bsv`、`Types.bsv`；Bluesim VCD 输出一致性
- [ ] `testsuite/bsc.lib/BRAM/BRAM0Test/BRAM0Test.exp` — 共享 `-verilog -elab` 在原生 Windows 串行运行超过 300 秒；整份脚本暂不迁移
- [x] `testsuite/bsc.lib/BRAM/Lat/Lat.exp` — `Latency1Port.bsv`
- [x] `testsuite/bsc.if/split-execution/TurboFIFO/attribute/execute.exp` — `TurboFIFO.bsv`；Bluesim/Icarus VCD
- [x] `testsuite/bsc.if/split-execution/TurboFIFO/original/execute.exp` — `TurboFIFO.bsv`；Bluesim/Icarus VCD
- [x] `testsuite/bsc.bsv_examples/AES/aes.exp` — 6 个本地 BSV、4 个 vector runtime fixture、4 个额外生成模块；heavy
- [x] `testsuite/bsc.bsv_examples/FP/FP.exp` — `FloatingPoint.bsv`
- [x] `testsuite/bsc.bsv_examples/GlibcRandom/GlibcRandom.exp` — `GlibcRandom.bsv`
- [x] `testsuite/bsc.bsv_examples/mimo/mimo.exp` — 部分 `-no-aggressive-conditions`

### 可静态归一化的简单 Tcl 包装：2 个

- [x] `testsuite/bsc.verilog/positivereset/SyncReset/SyncReset.exp` — 将临时 `BSC_OPTIONS` 展开为 case-local 生成与链接参数 `-reset-prefix RESET_P -D BSV_POSITIVE_RESET`
- [x] `testsuite/bsc.real/evaluator/undef/undef.exp` — `$vtest` 条件直接映射为 `Requirement::VerilogEnabled`

## 已完成：artifact/text assertion 第一批

通用 `ArtifactAssertion` / `TextAssertion` 已覆盖固定字符串、匹配行数、多行正则、诊断计数及文件存在性检查；alignment 会把 `.exp` assertion 与 Rust 声明逐项、逐数量核对。

- [x] `testsuite/bsc.typechecker/mismatch/mismatch.exp` — 12 个 compile contract；覆盖 `find_n_strings`
- [x] `testsuite/bsc.names/portRenaming/conflicts/enableReady/enableReady.exp` — 6 个 compile contract；覆盖 `find_n_emsg`
- [x] `testsuite/bsc.names/portRenaming/moduleArgs/moduleArgs.exp` — 39 个 compile contract；覆盖 generated Verilog 的 `string_occurs` / `string_does_not_occur`
- [x] `testsuite/bsc.typechecker/context-errors/context-errors.exp` — 59 个 compile contract；19 个 compiler golden 和 3 个位置正则 assertion
- [x] `testsuite/bsc.typechecker/constructors/constructors.exp` — 29 个 compile contract、2 个 shared-elaboration scenario、4 个 Bluesim/Icarus simulation contract

classic `test_c_veri*` helper 的静态权重补齐后，contract inventory 会同时统计 `.bs` 双后端 contract；因此全仓库可静态识别的 contract 分母由旧估算 4368 校正为 4600。

## 已完成：统一 artifact comparison 与 schedule contract

共享 `artifact` 模块现在由 compile 和 simulation pipeline 共同使用，支持文件存在、文本断言以及 exact、golden-output、Verilog 三种 actual/expected 比较。`VerilogSchedule` compile mode 对齐 `compile_verilog_schedule_pass`，alignment 同步核对 `compare_file` 和 `compare_verilog`。

- [x] `testsuite/bsc.typechecker/error_recovery/error_recovery.exp` — 11 个 compile contract；包含 `-continue-after-errors` 后生成 Verilog比较
- [x] `testsuite/bsc.lib/CompletionBuffer/CompletionBuffer.exp` — 1 个 schedule compile、2 个 shared simulation contract
- [x] `testsuite/bsc.lib/Cntrs/Cntrs.exp` — 2 个 compile、6 个 shared simulation contract；比较 `.sched` 与 generated Verilog
- [x] `testsuite/bsc.misc/fwrite/fwrite.exp` — 7 个 compile fail、23 个 backend-specific simulation contract；逐 backend 比较 `*.dat.out` 副产物
- [x] `testsuite/bsc.lib/SShow/SShow.exp` — 1 个指定 top 的 Verilog compile contract；比较 elaboration 生成的 `sysTestSShow.out`
- [x] `testsuite/bsc.scheduler/paths/paths.exp` — 56 个 Verilog compile contract；56 条正向、77 条负向 generated RTL regex assertion；新增 `find_regexp_fail` 对齐
- [x] `testsuite/bsc.syntax/bsv05/import-foreign/import-foreign.exp` — 74 个 compile contract；覆盖 warning count、compiler golden、generated RTL regex 和 2 个 `compare_verilog`；后者同时通过 `sv-parser` syntax smoke

识别 `compile_verilog_schedule_pass` 后，全仓库可静态识别 contract 分母由 4600 校正为 4672；新增的 72 个 contract 来自此前被归为自定义 Tcl 的 schedule 调用，并非本批凭空增加测试。

## 明确阻塞，不得机械迁移

| Origin | 阻塞原因 |
| --- | --- |
| `testsuite/bsc.evaluator/performance/performance.exp` | `BNotShared.bsv` 在原生 Windows codegen 超过 300 秒；注释预期数秒完成，需先确认编译器性能回归，不应简单放宽超时 |
| `testsuite/bsc.lib/FloatingPoint/FloatTest.exp` | `FloatTest.bsv` 的共享 `-verilog -elab` 在原生 Windows 串行运行超过 600 秒；不能只保留同一 `.exp` 中其余 7 个通过的 contract |
| `testsuite/bsc.lib/BRAM/BRAM0Test/BRAM0Test.exp` | `BRAM0Test.bsv` 的共享 `-verilog -elab` 在原生 Windows 串行运行超过 300 秒；不能用 backend-specific 生成绕开 upstream 的共享 elaboration 语义 |
| `testsuite/bsc.bugs/bluespec_inc/b925/b925.exp` | Bluesim XFAIL / bug gate，当前 Requirement 无法表达 |
| `testsuite/bsc.bluesim/operators/operators.exp` | 同时存在 Bluesim 和 Verilog bug gate |
| `testsuite/bsc.if/split-execution/2x2-switch-split/switch.exp` | 手工 interactive Bluesim 和 cycle assertion |
| `testsuite/bsc.if/split-execution/2x2-switch/switch.exp` | 手工 interactive Bluesim 和 cycle assertion |
| `testsuite/bsc.lib/DefaultValue/DefaultValue.exp` | `compile_pass_warning` 尚未建模 |
| `testsuite/bsc.lib/FShow/FShow.exp` | `compile_pass_warning` 尚未建模 |
| `testsuite/bsc.lib/oint/oint.exp` | `compile_verilog_pass_no_warning_bug` bug gate |
| `testsuite/bsc.bugs/bluespec_inc/b1666/b1666.exp` | Verilog 预期 link failure，不能截断为 Bluesim-only |
| `testsuite/bsc.lib/getput/getput.exp` | 动态 Icarus 探测和额外 `find_n_strings` assertion |
| `testsuite/bsc.bsv_examples/bsvfifo/bsvfifo.exp` | copy/erase/manual link/simulation 流程 |
| `testsuite/bsc.bugs/bluespec_inc/b535/b535.exp` | copy/erase/manual link/simulation 流程 |
| `testsuite/bsc.arrays/arrays.exp` | 条件分支和 `compile_verilog_fail_bug` |
| `testsuite/bsc.mcd/ModArgs/ModArgs.exp` | 多种未支持 no-warning/no-internal-error contract |
| `testsuite/bsc.driver/gensign/gensign.exp` | dumpbi/dumpbo 和字符串计数流程 |
| `testsuite/bsc.mcd/Reset/Reset.exp` | 动态分支、正则和 simulation 混合流程 |
| `testsuite/bsc.names/portRenaming/enableTests/enableTests.exp` | compile 后还有 no-main link contract |
| `testsuite/bsc.compile/compile.exp` | 动态替换 fixture 和延迟流程 |
| `testsuite/bsc.verilog/foreign_module/foreign_module.exp` | 活动 fail source 缺失，当前 fixture contract 无法表达 |

## 维护流程

每完成一批：

1. 更新本文件对应 checkbox 和必要说明。
2. 运行 `pixi run just test-alignment`。
3. 运行 `pixi run just inventory-update`，从同一 alignment registry 重建完整剩余清单。
4. 运行 `pixi run just inventory-check`，CI/提交前确认文档没有过期。
5. 运行完整 `pixi run just test`。
