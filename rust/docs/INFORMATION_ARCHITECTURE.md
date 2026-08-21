# Information Architecture：BSC Rust 文档

本文记录 `rust/docs/` 的信息架构。它不是产品设计文档，而是文档内容的导航、归属和增长规则。

## Site Map

- Rust 文档首页 `rust/docs/README.md`
  - 当前测试架构 `rust/docs/ARCHITECTURE.md`
  - 缓存与磁盘占用 `rust/docs/CACHE.md`
  - Upstream 边界 `rust/docs/UPSTREAM.md`
  - Testsuite 迁移规则 `rust/docs/MIGRATION.md`
  - 后续路线 `rust/docs/ROADMAP.md`
  - 历史设计 `rust/docs/history/TEST_PLAN.md`
- Rust workspace 入口 `rust/README.md`
- 测试操作入口 `rust/tests/README.md`
- 动态剩余清单 `rust/tests/REMAINING.md`
- Crate 局部文档 `rust/util/*/README.md`

文档导航最多两层：`rust/docs/<topic>.md` 或 `rust/docs/history/<topic>.md`。不要继续增加更深目录。

## Navigation Model

- **Primary navigation**：`rust/docs/README.md` 按“运行测试、理解架构、继续迁移”三个高频任务分流。
- **Secondary navigation**：每篇主题文档末尾链接最相关的下一篇，而不是复制完整目录。
- **Utility navigation**：crate README 提供命令和局部实现入口；`REMAINING.md` 提供动态状态。
- **History navigation**：历史设计只从当前架构或 docs 首页进入，避免新人误把旧统计当现状。

## Content Hierarchy

### 文档首页

1. 按任务选择阅读路径——读者最先需要知道“我该看哪一篇”。
2. 文件地图——帮助 reviewer 找到文档实际位置。
3. 内容归属和事实优先级——防止文档再次散落或互相冲突。

### 架构文档

1. BSC、Bluesim、Icarus、testsuite 的背景。
2. 一条测试从 `.exp` 到 Rust runner 的完整路径。
3. `complete` / `blocked`、fixture、artifact 和 provenance。
4. Importer、workflow composition、缓存和目标图模型等维护细节。

### 缓存文档

1. 先区分“Git 生成物、持久缓存、临时文件”。
2. 每种缓存保存与不保存的材料。
3. Cache key 和失效规则。
4. 磁盘体积、清理命令和误删风险。

### 迁移文档

1. 不可违反的不变量。
2. 从 `REMAINING.md` 选择批次的工作流。
3. 定向测试、生成物和最终门禁。
4. 长期路线放在 `ROADMAP.md`，动态数量不复制。

## User Flows

### 新开发者第一次运行测试

1. 从 `rust/docs/README.md` 进入“构建或运行测试”。
2. 阅读 `rust/tests/README.md` 获取 Just 命令。
3. 测试较慢或磁盘增长时阅读 `CACHE.md`。
4. 需要理解失败计划时进入 `ARCHITECTURE.md` 和 `REMAINING.md`。

### Reviewer 审查 importer 改动

1. 从 `ARCHITECTURE.md` 确认 typed/fail-closed 边界。
2. 从 `MIGRATION.md` 确认该批次是否通用、是否保持 testsuite 零改动。
3. 检查 contracts/plans/remaining 的确定性 diff。
4. 若出现 testsuite diff，转到 `UPSTREAM.md`，默认拒绝本地修改。

### 维护者分析磁盘占用

1. 从 `CACHE.md` 区分 scenario、BSC result、sccache 和临时目录。
2. 根据目录和 cache key 判断是否需要保留。
3. 使用 `test-prune` 清理临时目录；清理持久缓存前确认没有测试运行。

## Naming Conventions

| 概念 | 统一名称 | 说明 |
| --- | --- | --- |
| 官方仓库内容 | upstream | 不用“原版”“旧版”混指 |
| `.exp` 转换组件 | importer | lowerer 和 composer 是其内部阶段 |
| 运行时 JSON | Test Plan | 不称为 manifest |
| Importer 中间层 | manifest / importer IR | runner 不读取 |
| 测试输入 | fixture | 与执行后 artifact 区分 |
| 执行产生的文件 | artifact | 明确 inputs/outputs/removals |
| 无法安全迁移 | blocked | 不称为 skipped 或 failed |
| 可删除运行目录 | temporary workspace/artifacts | 不称为 cache |
| 可复用持久结果 | cache | 必须有明确 key 和失效规则 |

## Component Reuse Map

| 内容组件 | 使用位置 | 维护规则 |
| --- | --- | --- |
| 常用 Just 命令 | tests README、migration、cache | tests README 为主，其他文档只保留场景相关子集 |
| 动态迁移数量 | `REMAINING.md`、plans index | 手写文档只链接，不复制长期基线 |
| 术语解释 | architecture、IA | architecture 面向读者，IA 约束命名 |
| Upstream 零改动规则 | upstream、migration、architecture | `UPSTREAM.md` 为规范来源，其他文档摘要并链接 |
| 缓存目录和清理 | cache、tests README | `CACHE.md` 为完整来源，tests README 保留速查 |

## Content Growth Plan

- 新的跨 crate 主题直接增加 `rust/docs/<TOPIC>.md`，并在首页按用户任务挂接。
- 单 crate 实现细节优先扩充 crate README，不在 docs 创建重复页面。
- 动态报告继续由生成器维护，不把每次迁移数字追加进历史段落。
- 已被当前架构替代但仍有背景价值的文档移入 `history/`；无价值的重复内容直接删除。
- `rust/docs/` 顶层建议保持在 8～10 篇以内；超过后先合并重叠主题。

## Path Strategy

- 当前文档：`rust/docs/<UPPER_SNAKE_TOPIC>.md`。
- 历史文档：`rust/docs/history/<UPPER_SNAKE_TOPIC>.md`。
- Crate 文档：`rust/<crate-or-area>/README.md`。
- 自动生成状态：保持生成器既有固定路径，不为整理外观而移动。
- 所有链接使用仓库内相对路径，移动文件后必须做链接存在性检查。
