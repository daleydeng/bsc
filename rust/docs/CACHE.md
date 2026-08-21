# BSC Rust 缓存与中间产物

本文回答四个问题：项目保存了什么、为什么保存、什么变化会使缓存失效，以及如何安全清理。

## 1. 先区分四类磁盘内容

| 类型 | 典型位置 | 是否提交 Git | 是否可删除 | 主要用途 |
| --- | --- | --- | --- | --- |
| 生成的测试描述 | `rust/tests/contracts/`、`rust/tests/plans/` | 是 | 不应随意删除 | 审查 importer 输出，作为 runner source of truth |
| 持久测试缓存 | `.pixi/cache/rust-tests/` | 否 | 可以 | 避免重复执行昂贵的 canonical Test Plan 场景 |
| 编译器缓存 | `SCCACHE_DIR` | 否 | 可以 | 复用 Rust 和 Bluesim C/C++ 编译结果 |
| 一次运行的临时材料 | `.pixi/tmp/rust-test-*`、benchmark logs | 否 | 可以 | 当前失败诊断、workspace 和长日志 |

`rust/tests/plans` 虽然是“中间生成物”，但不是缓存。它经过版本控制和审查，是 Rust runner 的唯一运行时计划来源。

## 2. 当前体积大约是多少

一次本机盘点结果如下；缓存会随测试覆盖和工具链变化增长，因此只把这些数字视为量级参考：

| 内容 | 参考体积 |
| --- | ---: |
| Typed contract manifest | 约 5.7 MiB |
| 860 份 Test Plans、index 和 schema | 约 24.3 MiB |
| Scenario result cache | 约 1.0 GiB |
| 临时 test workspace | 约 214 MiB |
| 临时 test artifacts/log | 约 73 MiB |

提交到 Git 的生成描述合计约 **30 MiB**。本地 scenario cache 约 **1.0 GiB**，主要用于把全量热测试从几十分钟降到几分钟。

`sccache` 大小不能写成固定值：默认目录是 `.pixi/cache/sccache`、默认上限为 `10G`，但用户设置的 `SCCACHE_DIR` 和 `SCCACHE_CACHE_SIZE` 优先，可能指向仓库外的共享大缓存。

## 3. Scenario result cache

位置：

```text
.pixi/cache/rust-tests/scenario-results/v1/
└── <sha256-key>/
    ├── .complete
    └── assertions/
        └── <stage>/<operation>/...
```

### 保存什么

- 场景成功完成的 `.complete` 标记；
- 每个 assertion 执行时的 actual/expected 文件快照；
- 被断言直接检查的文本、Verilog、C++ 或 VCD 文件。

大型 VCD 本身如果是 assertion 对象，就必须进入快照。这通常是 scenario cache 中最大的材料。

### 不保存什么

- 整个 scenario workspace；
- 普通 `.bo`、`.bi`、`.ba`；
- object、生成的 executable；
- 没有被 assertion 引用的 Verilog/C++ 文件；
- 临时编译目录和普通工具日志。

cache hit 不恢复可变 workspace，而是从只读 snapshot 重新执行 assertions。这样既能跳过昂贵工具调用，又不会让测试修改 persistent cache。

### Cache key 包含什么

Scenario key 是内容寻址的 SHA-256，包含：

1. Cache 格式 schema；
2. 操作系统和 CPU 架构；
3. `bsc` executable 内容；
4. 整个 `inst/lib` / `BLUESPECDIR` 文件树内容；
5. Scenario 使用的每个 fixture 路径和文件内容；
6. Runner 传入的 fingerprint parts，包括 executor schema、plan ID 和序列化 scenario；
7. 工具 argv；
8. `PATH`、`BLUESPECDIR` 和固定 `BSCTEST=1`；
9. 会影响 BSC/本机编译的环境变量，例如 `BSC_OPTIONS`、`BSC_PATH`、`CC`、`CXX`、flags、`MAKE`、`SYSTEMC`、locale。

因此，源码、plan、BSC、安装库、参数或相关环境变化都会自然产生 cache miss，不需要手动判断旧 entry 是否兼容。

## 4. `sccache`

`sccache` 缓存的是**编译器输出**，不是 BSC 测试结果：

- Rust `rustc` 的可缓存编译结果；
- Bluesim 和其他 C/C++ 编译产生的 object；
- 与编译命令、源文件、编译器和环境相关的 metadata。

Pixi/xtask 默认设置：

```text
RUSTC_WRAPPER=<Pixi sccache>
CXX=sccache c++
SCCACHE_DIR=.pixi/cache/sccache
SCCACHE_CACHE_SIZE=10G
```

如果调用者已经设置这些环境变量，则保留调用者配置。因此 `just sccache-clear` 清理的是**当前生效的 `SCCACHE_DIR`**，它可能位于仓库外并被多个项目共享，执行前应先确认。

常用命令：

```sh
pixi run just sccache-stats
pixi run just sccache-clear
```

`test-prune` 不会清理 `sccache`。

## 5. 临时 workspace、artifacts 和日志

这些目录不是 cache：

```text
.pixi/tmp/rust-test-work/       场景实际执行 workspace
.pixi/tmp/rust-test-artifacts/  工具日志、diff 和失败诊断
.pixi/tmp/benchmarks/           主动重定向的长测试日志
```

失败场景保留 workspace 和 artifacts，便于查看当时真正生成的文件。完整成功后的临时目录也可能暂时存在，但不会参与下一次 cache hit。

清理命令：

```sh
pixi run just test-prune
```

`test-prune` 只删除：

- `.pixi/tmp/rust-test-work`；
- `.pixi/tmp/rust-test-artifacts`。

它明确保留：

- Cargo target；
- scenario result cache；
- `sccache`；
- `.pixi/tmp/benchmarks` 下的人为保留日志。

## 6. 缓存写入为什么不容易损坏

Scenario result cache 采用以下发布规则：

1. 在 cache root 下创建唯一 `.tmp-*` 目录；
2. 写完全部内容；
3. 最后写并 `sync` `.complete`；
4. 用目录 rename 原子发布；
5. 并发 writer 已先发布同 key 时，丢弃自己的临时目录；
6. lookup 发现 entry 不完整或结构非法时删除并按 miss 处理。

读取时拒绝 symlink 和非普通文件/目录，避免 cache 内容把 runner 引向缓存根之外。

## 7. 何时会 miss，何时需要手动清理

正常情况下不需要因为代码变化手动清理。Key 变化会自动产生新 entry。

| 情况 | 行为 |
| --- | --- |
| Plan、fixture、BSC、`inst/lib`、argv 或相关环境变化 | 自动 miss，旧 entry 仍占磁盘 |
| Cache 格式不兼容 | 代码升级 namespace/schema，自动使用新 key 空间 |
| 中断写入 | 无 `.complete`，下次 lookup 删除 |
| 只想验证不命中测试结果 cache | `BSC_TEST_CACHE=0` 或 `pixi run just test-cold` |
| 临时失败材料太多 | `pixi run just test-prune` |
| scenario cache 占用太大 | 无测试运行时删除 `.pixi/cache/rust-tests` |
| 编译缓存占用太大 | 确认当前 `SCCACHE_DIR` 后运行 `just sccache-clear` |

删除持久 cache 不会破坏仓库或测试正确性，只会让下一次运行重新计算，可能重新花费几十分钟。

## 8. 什么不应该进入 cache

新增 cache 行为时保持以下边界：

- 不保存未被声明的 host 文件；
- 不把整个 testsuite 目录作为隐式输入；
- 不缓存失败场景为“完成”；
- 不让 XFAIL 吞掉进程启动、timeout 或文件损坏；
- 不在 key 中漏掉能影响结果的 tool、fixture、plan 或 environment；
- 不让 cache hit 提供可修改的 persistent 文件；
- 不为了提高命中率放松 artifact contract。

相关架构背景见 [`ARCHITECTURE.md`](ARCHITECTURE.md)。
