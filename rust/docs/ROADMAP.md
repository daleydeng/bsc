# 下一步迁移

剩余测试的唯一动态盘点是 [`REMAINING.md`](REMAINING.md)。执行：

```text
BSC_JOBS=1 pixi run just inventory-check
```

当前基线：860 plans，762 complete，98 blocked。Test Plan schema v23 已让每个 operation 显式声明 artifact inputs、outputs 与 removals，并为严格闭合的 `BSC_OPTIONS` save/append/restore scope 提供 scenario-scoped typed overlay；下一步按 `REMAINING.md` 的高杠杆 blocker 类别扩展通用组合器，而不是为单个 origin 写特判。完整的当前设计和已知边界见 [`ARCHITECTURE.md`](ARCHITECTURE.md)。

优先方向：

1. 以 source order、guard、artifact 和 barrier 构建统一的 versioned action graph；路径覆写必须用 generation identity，而非裸 path 或“最近 producer”。
2. 让 `CheckBindings` 与 workflow composer 直接消费 operation artifact contracts，删除 importer 内残余的平行产物推导函数。
3. 基于唯一 producer/path-version/guard/order 证明组合 transfer/erase/create-directory/compile-object/link/run actions；候选组件必须原子提交。
4. 为 artifact-flow 增加小型 hermetic fixture tests，覆盖歧义、barrier、guard、copy/move/remove、跨 backend 链接和 source overwrite。
5. 将 OVL、SystemC、Bluetcl 和 SchedulerSat 保持为独立 typed runner family，不污染 generic core；动态环境和 tool capability 条件需要专门的 typed model。
6. 每次迁移后重新生成 contract、plan 和 remaining inventory，并用 RTK 串行执行校验与新增 complete plan。

完整规则见 [`MIGRATION.md`](MIGRATION.md)，Test Plan 设计见 [`TEST_PLAN.md`](TEST_PLAN.md)。
