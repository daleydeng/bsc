# Rust 重写计划索引

状态：规划中
更新时间：2026-08-21

## 文档

- [`OVERALL.md`](OVERALL.md)：BSC、Bluesim、Bluetcl 的总体重写顺序、边界和全局门禁。
- [`BLUESIM.md`](BLUESIM.md)：第一阶段 Bluesim 的独立实施计划。

## 当前决策

完整替换顺序：

```text
Bluesim → BSC → Bluetcl
```

实际迁移顺序：

```text
Bluesim ABI/kernel
→ Rust bluesim CLI/session + 迁移期 Bluetcl adapter
→ BSC driver 与阶段协议
→ BSC 语义流水线
→ Bluetcl compiler-query 命令
```

## 当前行动

从 [`BLUESIM.md`](BLUESIM.md) 的 M0 开始：选择最小 fixture，定义 versioned SimIR vertical slice，由 legacy Haskell pipeline 导出首个 `.bsim`，再让 Rust `bluesim` binary 直接加载。闭环通过前不创建完整 runtime 框架。
