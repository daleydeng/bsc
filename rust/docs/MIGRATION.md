# Testsuite 迁移规则

## 目标

将 upstream `testsuite/**/*.exp` 的可观察契约静态导入为版本化 Test Plan，并由唯一 Rust runner 执行。迁移对象包括 fixture、参数、退出状态、诊断、产物、仿真、VCD 和 golden/assertion；不实现或嵌入 Tcl 解释器。

## 不变量

1. `testsuite/` 零改动。
2. `.exp` 只在 import/sync 阶段读取；runtime 只执行 `rust/tests/plans`。
3. 只允许 typed operation vocabulary；shell、eval 和未知动态语义不可进入计划。
4. 无法静态证明的语义产生 provenance diagnostic，并使 plan `blocked`。
5. 不按 plan ID 或 origin 写特判。
6. fixture、producer/consumer、guard 和 artifact flow 必须可静态追踪。
7. 不保留手写 registry、legacy runner 或目录兼容层。

## 迁移循环

```text
Tree-sitter Tcl CST
→ typed command/event stream
→ generic artifact-flow composition
→ Test Plan
→ canonical runner
```

每批迁移：

1. 根据 [`REMAINING.md`](REMAINING.md) 选择 blocker 类别或 candidate，不再人工全仓盘点。
2. 在 `rust/util/testsuite-manifest` 泛化 lowerer/composer；避免 origin 特判。
3. 为 importer/model/runtime 添加最小定向单测。
4. 更新生成物并运行门禁：

```text
BSC_JOBS=1 pixi run just contracts-update
BSC_JOBS=1 pixi run just plans-update
BSC_JOBS=1 pixi run just inventory-update
BSC_JOBS=1 pixi run just contracts-check
BSC_JOBS=1 pixi run just plans-check
BSC_JOBS=1 pixi run just plans-audit
BSC_JOBS=1 pixi run just inventory-check
```

5. 定向执行新增 complete plan，再执行单线程全量 `just test`。
6. 用 `git diff --exit-code -- testsuite` 确认 upstream fixture 未改动。

## 完成标准

迁移完成时应满足：

- 860/860 plan 为 `complete`，或剩余 skip 由明确平台 capability 表达。
- `REMAINING.md` 无 unsupported/import blocker。
- canonical runner 覆盖所有 active upstream contracts。
- repository 中不需要 Tcl/DejaGNU 才能执行测试。
