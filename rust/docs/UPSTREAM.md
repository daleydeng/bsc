# Upstream 与 `testsuite/` 边界

## 结论

正常情况下，本分支不应修改 `testsuite/`。

`testsuite/` 是 BSC 官方测试契约和 fixture 的镜像。Rust importer 的职责是理解它，而不是把它改成更容易理解的形式。最终门禁要求：

```sh
git diff --exit-code -- testsuite
```

## 为什么必须保持零改动

1. **方便合并 upstream**：本分支需要长期跟随 `B-Lang-org/bsc`，测试目录本地分叉会持续制造冲突。
2. **避免改变测试问题**：改诊断编号、删除 golden 或重命名输入，可能让 Rust runner 通过，但已经不是在执行同一份测试契约。
3. **暴露 importer 缺口**：如果 `.exp` 有 XFAIL、动态 helper 或复杂 artifact flow，应该扩展 typed model，不能删掉难懂部分。
4. **保持 review 可比性**：reviewer 应能把生成 Test Plan 与官方 `.exp` 一一对照。
5. **防止自证正确**：同时修改测试和实现很容易让错误行为得到新的“正确答案”。

## 不允许用 testsuite 修改解决的问题

- Importer 暂时不支持某个 helper；
- Rust runner 还不能执行某类 workflow；
- 当前 BSC 输出的诊断编号与 upstream 预期不同；
- 某个 known-bug golden 导致 XPASS/XFAIL 处理复杂；
- fixture 路径或扩展名看起来像 upstream typo；
- 为了把 `blocked` 改成 `complete`；
- 为了让 Windows 定向测试先通过。

这些情况分别应通过 typed lowering、operation model、artifact graph、XFAIL semantics、明确 blocker 或 upstream 修复处理。

## 发现 upstream 测试本身有问题时

推荐流程：

1. 在未修改 `testsuite/` 的情况下记录可复现行为；
2. 确认问题来自 upstream fixture/contract，而不是本分支 importer；
3. 向 upstream 提交 issue 或 PR；
4. 修复进入 `upstream/main` 后，通过正常 merge/pull 带回；
5. 重新生成 contracts、plans 和 remaining inventory；
6. 确认本分支仍然没有额外 testsuite diff。

在等待 upstream 修复期间，对应 Test Plan 保持 `blocked`，并用 import diagnostic 或 blocker inventory 解释原因。不要在本地静默“纠正”测试。

## 什么情况可以出现 testsuite diff

只有两类临时情况：

1. **本地诊断实验**：开发者短暂修改 fixture 以确认根因，但修改不得进入提交，实验结束必须恢复。
2. **正在准备 upstream PR**：改动本身就是准备贡献给官方的独立补丁；它应与 Rust importer 迁移改动分开 review 和提交。

如果确实需要在本分支长期携带例外，必须先得到明确决策，并记录：

- upstream issue/PR URL；
- 涉及文件；
- 为什么无法等待 upstream；
- 与官方语义的差异；
- 删除条件和负责人。

当前设计目标是 **没有长期例外**。

## 本次迁移审查中发现并撤回的调整

以下本地调整曾出现在工作树中，现已撤回。记录它们是为了说明边界，而不是为其建立兼容规则。

| 调整 | 可能解决的表面问题 | 为什么拒绝 |
| --- | --- | --- |
| `Bug437BSV.bs` 改名为 `Bug437BSV.bsv` | 让 `b437.exp` 引用的显式 `.bsv` 路径实际存在 | Upstream 自初始发布就存在扩展名不一致；本地重命名会掩盖 upstream discrepancy，应由 upstream 修复或保持 blocked |
| `b437.exp` 的预期诊断 `T0020` 改为 `T0080` | 匹配当前编译器实际输出，使 assertion 通过 | 这直接改变官方测试契约；如果行为变化合理，应由 upstream 更新，否则应报告回归/XFAIL |
| 删除 `Imported_Modules.exp` 中 `compare_file_bug ... 770` | 绕开 known-bug golden 和 guard 组合，使 plan 更容易 complete | 删除了真实的 XFAIL/golden 检查；正确做法是完善 `compare_file_bug`、guard 和 producer binding |

其中 `Bug437BSV.bsv` 引用与仓库中的 `Bug437BSV.bs` 不一致，是值得单独向 upstream 报告的问题，但不能因此在 importer 中按 origin 特判或修改本地 testsuite。

## Review checklist

涉及测试迁移的提交至少检查：

```sh
BSC_JOBS=1 pixi run just contracts-check
BSC_JOBS=1 pixi run just plans-check
BSC_JOBS=1 pixi run just plans-audit
BSC_JOBS=1 pixi run just inventory-check
git diff --check
git diff --exit-code -- testsuite
```

若最后一条失败，先解释每一处 diff；默认动作是撤回，而不是补一段说明让它留下。
