# BSC Rust 文档

这里集中维护跨 Rust crate 的长期文档。第一次接触 BSC 或本分支时，从本页开始，不需要先理解 Tcl、DejaGNU 或 importer 内部实现。

## 按你的目标阅读

### 我只想构建或运行测试

1. [`../README.md`](../README.md)：Rust workspace 目录和职责。
2. [`../tests/README.md`](../tests/README.md)：常用 Pixi/Just 测试命令。
3. [`CACHE.md`](CACHE.md)：缓存了什么、占用多大、如何失效和清理。

### 我想理解为什么要迁移 testsuite

1. [`ARCHITECTURE.md`](ARCHITECTURE.md)：从 BSC 基础概念开始解释完整测试架构。
2. [`UPSTREAM.md`](UPSTREAM.md)：为什么 `testsuite/` 必须保持零改动，以及发现上游问题时怎么处理。
3. [`history/TEST_PLAN.md`](history/TEST_PLAN.md)：早期 Test Plan 设计记录，仅用于了解历史。

### 我要继续迁移 blocked tests

1. [`../tests/REMAINING.md`](../tests/REMAINING.md)：自动生成的当前 blocker 清单。
2. [`MIGRATION.md`](MIGRATION.md)：迁移不变量、工作循环和完成标准。
3. [`ROADMAP.md`](ROADMAP.md)：长期重构方向和优先级。
4. [`../util/testsuite-manifest/README.md`](../util/testsuite-manifest/README.md)：Tcl 静态前端和 importer 细节。
5. [`../util/test-plan/README.md`](../util/test-plan/README.md)：Test Plan 数据模型。

## 文档地图

```text
rust/docs/
├── README.md                    本页：总入口
├── ARCHITECTURE.md              当前系统如何工作
├── CACHE.md                     缓存、临时文件和磁盘占用
├── UPSTREAM.md                  upstream 边界与 testsuite 零改动政策
├── MIGRATION.md                 迁移规则
├── ROADMAP.md                   后续方向
├── INFORMATION_ARCHITECTURE.md  文档组织与维护规则
└── history/
    └── TEST_PLAN.md             历史设计背景
```

留在代码附近的文档：

- `rust/tests/README.md`：测试 crate 的操作入口；
- `rust/tests/REMAINING.md`：自动生成，不手改；
- `rust/util/*/README.md`：各 crate 的局部实现说明。

## 内容归属规则

- 跨多个 crate 的解释、架构和政策放在 `rust/docs/`。
- 单个 crate 的 API、命令或实现细节放在该 crate 的 `README.md`。
- 动态数量和 blocker 只放在自动生成的 `rust/tests/REMAINING.md` 或 plans index，不在多篇手写文档重复维护。
- 历史设计不与当前架构混写；保留时放入 `history/` 并明确标注可能过时。
- `testsuite/` 是 upstream 内容，不用它承载本分支文档。

## 当前事实的优先级

文档与代码不一致时，按以下顺序判断：

1. Rust model、importer 和 runner 代码；
2. 自动生成且通过 check 的 plans、schema、contracts 和 `REMAINING.md`；
3. `ARCHITECTURE.md`、`CACHE.md` 等当前文档；
4. `history/` 中的历史记录。

发现手写文档中的 schema 版本或数量过期时，应更新或删除该快照，不要再增加一份新的统计来源。
