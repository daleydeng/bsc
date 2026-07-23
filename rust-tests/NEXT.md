# Rust testsuite migration plan

本文件记录已经人工核对过的迁移候选、实施顺序和阻塞原因。完整且自动更新的剩余 `.exp` 清单见 [`REMAINING.md`](REMAINING.md)。

## 当前基线

- 已迁移来源：310/860
- 尚未迁移来源：550
- 已迁移静态 contract：1305/4161
- 尚未迁移静态 contract：2856
- 完全需要动态或自定义 Tcl 分析的脚本：250
- 最近稳定提交：`7af594bc Expand static compile and simulation coverage`

候选进入本文件的硬条件：必须完整迁移一整份 `.exp` 的全部活动 contract 和 assertion；不得只摘取 compile/simulation 调用；不得忽略 XFAIL、bug gate、generated artifact、手工 link/sim 或额外输出比较。

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

- [x] `testsuite/bsc.syntax/bsv05/interface/interface.exp` — 28；4 个 legacy `compile_fail source TAG` 的 TAG 保留在 options 参数
- [x] `testsuite/bsc.typechecker/assignment/assignment.exp` — 18；`StructUpdReg.bsv` 保留 legacy options；`RegStructWrite.bsv` 保留 frontend 与 Verilog 两个 contract
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

## 明确阻塞，不得机械迁移

| Origin | 阻塞原因 |
| --- | --- |
| `testsuite/bsc.evaluator/performance/performance.exp` | `BNotShared.bsv` 在原生 Windows codegen 超过 300 秒；注释预期数秒完成，需先确认编译器性能回归，不应简单放宽超时 |
| `testsuite/bsc.lib/FloatingPoint/FloatTest.exp` | `FloatTest.bsv` 的共享 `-verilog -elab` 在原生 Windows 串行运行超过 600 秒；不能只保留同一 `.exp` 中其余 7 个通过的 contract |
| `testsuite/bsc.lib/BRAM/BRAM0Test/BRAM0Test.exp` | `BRAM0Test.bsv` 的共享 `-verilog -elab` 在原生 Windows 串行运行超过 300 秒；不能用 backend-specific 生成绕开 upstream 的共享 elaboration 语义 |
| `testsuite/bsc.bugs/bluespec_inc/b925/b925.exp` | Bluesim XFAIL / bug gate，当前 Requirement 无法表达 |
| `testsuite/bsc.bluesim/operators/operators.exp` | 同时存在 Bluesim 和 Verilog bug gate |
| `testsuite/bsc.misc/fwrite/fwrite.exp` | 需要比较 simulation 生成的 `*.dat.out` 副产物 |
| `testsuite/bsc.if/split-execution/2x2-switch-split/switch.exp` | 手工 interactive Bluesim 和 cycle assertion |
| `testsuite/bsc.if/split-execution/2x2-switch/switch.exp` | 手工 interactive Bluesim 和 cycle assertion |
| `testsuite/bsc.lib/CompletionBuffer/CompletionBuffer.exp` | `compile_verilog_schedule_pass` 和 `.sched` artifact 比较 |
| `testsuite/bsc.lib/Cntrs/Cntrs.exp` | schedule / generated Verilog artifact 比较 |
| `testsuite/bsc.lib/DefaultValue/DefaultValue.exp` | `compile_pass_warning` 尚未建模 |
| `testsuite/bsc.lib/FShow/FShow.exp` | `compile_pass_warning` 尚未建模 |
| `testsuite/bsc.lib/oint/oint.exp` | `compile_verilog_pass_no_warning_bug` bug gate |
| `testsuite/bsc.bugs/bluespec_inc/b1666/b1666.exp` | Verilog 预期 link failure，不能截断为 Bluesim-only |
| `testsuite/bsc.lib/getput/getput.exp` | 动态 Icarus 探测和额外 `find_n_strings` assertion |
| `testsuite/bsc.bsv_examples/bsvfifo/bsvfifo.exp` | copy/erase/manual link/simulation 流程 |
| `testsuite/bsc.bugs/bluespec_inc/b535/b535.exp` | copy/erase/manual link/simulation 流程 |
| `testsuite/bsc.arrays/arrays.exp` | 条件分支和 `compile_verilog_fail_bug` |
| `testsuite/bsc.syntax/bsv05/import-foreign/import-foreign.exp` | 动态分支、正则 assertion 和 generated Verilog 比较 |
| `testsuite/bsc.typechecker/context-errors/context-errors.exp` | compile 后还有大量位置正则 assertion |
| `testsuite/bsc.scheduler/paths/paths.exp` | 核心语义是生成 RTL 路径检查 |
| `testsuite/bsc.names/portRenaming/moduleArgs/moduleArgs.exp` | 大量 `string_occurs` / `string_does_not_occur` assertion |
| `testsuite/bsc.mcd/ModArgs/ModArgs.exp` | 多种未支持 no-warning/no-internal-error contract |
| `testsuite/bsc.driver/gensign/gensign.exp` | dumpbi/dumpbo 和字符串计数流程 |
| `testsuite/bsc.mcd/Reset/Reset.exp` | 动态分支、正则和 simulation 混合流程 |
| `testsuite/bsc.typechecker/constructors/constructors.exp` | 额外字符串/正则 assertion 和 simulation |
| `testsuite/bsc.typechecker/mismatch/mismatch.exp` | tagged fail 后还有活动 `find_n_strings` assertion |
| `testsuite/bsc.names/portRenaming/enableTests/enableTests.exp` | compile 后还有 no-main link contract |
| `testsuite/bsc.compile/compile.exp` | 动态替换 fixture 和延迟流程 |
| `testsuite/bsc.lib/SShow/SShow.exp` | 比较 simulation 输出，不是 compiler golden |
| `testsuite/bsc.verilog/foreign_module/foreign_module.exp` | 活动 fail source 缺失，当前 fixture contract 无法表达 |
| `testsuite/bsc.names/portRenaming/conflicts/enableReady/enableReady.exp` | 额外 error-count assertion |
| `testsuite/bsc.typechecker/error_recovery/error_recovery.exp` | generated Verilog 比较 |

## 维护流程

每完成一批：

1. 更新本文件对应 checkbox 和必要说明。
2. 运行 `pixi run just test-alignment`。
3. 运行 `pixi run just inventory-update`，从同一 alignment registry 重建完整剩余清单。
4. 运行 `pixi run just inventory-check`，CI/提交前确认文档没有过期。
5. 运行完整 `pixi run just test`。
