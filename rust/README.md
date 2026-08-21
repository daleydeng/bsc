# Rust workspace

本目录集中维护 BSC 项目的 Rust 实现，避免与 upstream 的 Haskell/C++/Tcl 源码和 `testsuite/` 淨杂。

```text
rust/
├── tests/                 # canonical Test Plan runner、计划、契约清单与测试迁移状态
└── util/
    ├── test-plan/         # 版本化声明式 Test Plan 数据模型
    ├── testsuite-manifest/# .exp 的 Tree-sitter Tcl 静态前端与 importer
    ├── xtask/             # 构建、测试、同步和审计的复杂任务实现
    └── z3-bridge/         # BSC 到 Pixi-managed Z3 的稳定 C ABI bridge
```

未来如果逐步以 Rust 重写核心组件，使用并列目录，例如：

```text
rust/bsc/
rust/bluesim/
rust/bluetcl/
```

目录原则：

- `Justfile` 只提供简短、稳定的公开入口。
- `cargo xtask` 负责跨平台编排和文件系统操作。
- 可复用的数据模型与实现放入独立 crate。
- `rust/tests` 只执行版本化 Test Plan；运行期不读取或执行 Tcl。
- upstream `testsuite/` 保持零改动，便于合并 `upstream/main`。
- 不为已删除的旧测试 runner 或目录布局保留兼容层。
