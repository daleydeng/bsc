# BSC Test Plan 迁移设计

> **维护说明：** 本文保留 Test Plan 的规范背景和历史设计记录，其中的 schema、统计与部分 artifact 示例可能反映早期迁移阶段。当前架构、运行时语义、已知边界和动态基线请先读 [`ARCHITECTURE.md`](ARCHITECTURE.md)，剩余 blocker 以 [`REMAINING.md`](REMAINING.md) 为准。

## 目标

把 upstream DejaGNU/Tcl 测试从运行时依赖改造成一次性导入：

```text
upstream testsuite/**/*.exp
        │
        ▼
Tree-sitter Tcl + allowlisted static lowerer
        │
        ▼
versioned BSC Test Plan JSON
        │
        ▼
one canonical Rust runner
```

日常测试只读取 Test Plan；`.exp` 仅在同步 upstream、更新计划和审查差异时使用。
`testsuite/` 保持原样，便于长期 `pull upstream`。

## 当前 inventory

canonical inventory 不跟随 `testsuite/*/config` 目录符号链接：

```text
860 contract .exp
3 infrastructure .exp
931 Makefile
31 .tcl
23 .cmd
19 .pl
3 .sh
```

当前 importer 输出：

```text
Test Plan schema v18
860 plans
672 complete / 188 blocked
5148 scenarios / 5253 stages / 21772 operations
1575 diagnostics
```

Manifest schema v14 共恢复 3552 个 compile contract、2270 个 simulation contract、
24 个 external contract，以及 2 个 Bluesim sequence 和 142 个 Bluesim workflow。通用
`bsc.compile` lowerer 支持 frontend、Bluesim object、Verilog 和 schedule 四种 typed mode；compile 与 workflow 共享同一个有序
producer window，把紧随其后的 diagnostic、text、regex、golden 和 Verilog check 绑定到实际
产物。BSV 与 BH 本地 package/include 依赖会递归进入 fixture closure；源码中静态可恢复的运行时数据文件引用进入独立 `data` fixture closure。

Rust Z3 bridge、固定 solver worker 与共享 `ASDef` 的 SMT `define-fun` lowering 已使原始 K=64
`performance.exp` 完整通过；没有放宽 timeout 或降低 plan status。schema v18 除显式 generation
mode、simulation backend、统一 `simulation.run`、VCD validity assertion 和 scenario-scoped fixture
references 外，还要求每个 operation 声明 artifact inputs、outputs 与 removals。校验、fixture closure、
binding、cache fingerprint 和运行时后置条件统一消费该声明，不再各自推导产物文件名。

三个 infrastructure `.exp` 是：

```text
testsuite/site.exp
testsuite/config/unix.exp
testsuite/lib/bsc.exp
```

860 个 contract `.exp` 是测试契约源。其他文件不是第二套测试发现机制，但可能是源码、
golden、Bluetcl 程序、Bluesim command file、生成器、过滤器或 suite 基础设施，必须作为
传递依赖进入计划或审计清单。

## Canonical 文件布局

```text
rust/tests/plans/
├── schema.json
├── index.json
├── bsc.bluesim/
│   └── parallel/
│       └── parallel.test.json
└── bsc.bugs/
    └── bluespec_inc/
        └── b1894/
            └── b1894.test.json
```

每个 contract `.exp` 恰好对应一份 `.test.json`。路径保留完整 origin，不能仅使用 stem，
因为 upstream 存在大量重名 `.exp`。

## Schema 与版本

Rust model 位于 `rust/util/test-plan`：

- `schemaVersion` 独立版本化；
- Rust model 生成并提交 `schema.json`；
- JSON 使用 `deny_unknown_fields`；
- plan 记录 origin path 与 SHA-256；
- fixture 记录 path、role 与 SHA-256；
- operation 带原 `.exp` source span 和 procedure expansion；
- canonical pretty JSON 以 LF 和单个末尾换行保存。

Schema 升级必须显式修改版本，不允许静默改变旧计划语义。

## 完整与阻塞计划

批量 importer 必须为全部 860 个 origin 生成计划：

- `complete`：只包含 runner 已定义的有限 typed operations，可以执行；
- `blocked`：保留已静态恢复的 operation，同时用结构化 error diagnostics 精确解释缺口。

禁止忽略未知 Tcl 后仍标记 `complete`。Rust runner 必须拒绝执行 `blocked` plan。

## 执行模型

```text
Plans（Rayon 并行，`--jobs N`）
└── Plan
    └── Scenario（当前在 plan 内严格顺序）
        └── Stage（严格顺序，共享 workspace）
            └── Operation（严格保持 .exp 原顺序）
```

Assertion 是普通有序 operation，不再与 action 分栏。这样可以精确表达：

```text
fs.move
assert.regex
fs.remove
bsc.link
```

选择 sequence 后段 stage 时，runner 必须执行依赖前缀；只报告被选择 stage 的结果。
只有完整执行 scenario 时才能写入完整 workspace cache。

`bsc-test` 每个进程只创建一个共享 `TestPlanExecutor`，toolchain fingerprint 因此只计算一次。
完整 scenario 的 generation-cache hit 直接从 persistent cache 的只读 `files` snapshot 重放
assertion，不复制整个 workspace；cache miss 才创建运行目录并在成功后写入 snapshot。其他需要
可变工作区的 cache consumer 仍使用 copy restore，避免测试进程修改 persistent cache。

schema v18 的 scenario-scoped fixture references 与 operation artifact contracts 都参与 cache key 与 staging；plan 顶层 registry
继续按路径和 SHA-256 去重，但每个 scenario 只 stage 自己的 source、golden、operation 输入及
最初 380-plan 全热基准从约 63.3 秒降至约 3.9 秒；扩展到 462 个 complete plan 后的
Windows 全热运行约 20.7 秒，结果为 1944 passed、1 skipped、0 failed，cache 为
1910 hits、0 misses。

平台能力由 typed `requires` 声明。当前执行器不满足 requirement 时按 scenario 显式报告
`SKIP`，不改写路径、不部分执行，也不把平台限制误报成测试失败。例如 `parallel.exp` 的
`dir:with,many;spec#ial=char%acters` 在 POSIX 原样执行，在 Windows 由 `non_windows` 跳过。

## 有限 operation vocabulary

当前基础集合：

```text
bsc.compile
bsc.generate
bsc.link
simulation.run
fs.copy
fs.move
fs.remove
fs.mkdir
assert.exists
assert.text_contains
assert.text_absent
assert.regex
assert.regex_absent
assert.text_count
assert.regex_count
assert.diagnostic_count
assert.golden
assert.verilog
assert.vcd
assert.vcd_valid
```

后续按实际 upstream 语义增加 Bluetcl、scheduler、matrix、xfail 等 typed operation。每个操作
必须有明确输入、产物、退出码、timeout、resource 和 cache 语义。`bsc.compile` 已原生表达
frontend、Verilog、schedule、dependency mode、预期成功/失败和 stdout artifact；预期 compile
failure 是成功测试结果，成功 compile 必须验证 `.bo`。`bsc.generate` 显式区分 Bluesim、Verilog
和 shared elaboration，`bsc.link` / `simulation.run` 显式区分 Bluesim 与 Icarus；Icarus 输出噪声
由 runner 做固定归一化，VCD 由 Rust parser 验证，而不是执行 Tcl helper。

明确禁止：

```text
shell command string
Tcl/Python/JavaScript eval
隐式 &&、重定向和管道
任意动态脚本 operation
```

外部工具参数只能是 argv 数组。需要 Perl/Make helper 的历史测试先保持 `blocked`，随后将其
语义建模成专用 operation 或 Rust 实现，而不是给计划添加通用 shell escape hatch。

## Fixture 与传递依赖

计划引用但不复制 upstream fixture。至少记录：

- `.bsv/.bs/.v/.sv/.vhd/.c/.cpp` source；
- `.expected/.golden/.vcd` expected artifact；
- Bluetcl `.tcl` payload；
- Bluesim `.cmd` command file；
- active generator/filter/normalizer；
- fixture-building Makefile 及递归输入。

Importer 对已 lower 为可执行 `bsc.compile` 或 `bsc.generate` 的 source 做本地依赖闭包：
注释屏蔽后识别 BSV 的 `import Package::*`、classic BS 的 `import Package;`、BH 的
`import Package` / `import qualified Package`，以及 `` `include "file" ``，递归加入同 fixture
目录中唯一匹配的 `.bsv/.bs` package 和 include；本地 package 歧义、include 缺失或越界会使
plan 保持 `blocked`。未出现在 fixture 目录中的 import 仍由 BSC library path 提供。

同一 source closure 中的字符串字面量如果精确指向 fixture 目录内的现存普通文件，也会以
`FixtureRole::Data` 加入 scenario。对于 `name + "_file.txt"` 这类静态可判定的文件名组合，
importer 只匹配同目录中具有该后缀的现存文件；不扫描或 stage 整个 fixture 目录。这样既覆盖
RegFile、BRAM、AES 等运行时数据输入，也保持 scenario-scoped staging 和 cache key 的精确性。

任何传递依赖内容变化都必须使 `plans-check` 失败。fixture path 必须是 canonical 相对路径，
禁止绝对路径、`..` 和 runner 内部 `.bsc-test-plan` 命名空间。模型还拒绝大小写碰撞的 fixture、
comparison 自比较、不安全的 `bsc.link.top`、argv 中的绝对/父目录/内部命名空间路径和
Windows drive-relative 路径；Windows 非法路径必须由 `non_windows` requirement 显式限定。
CLI 与 runner 在读取 plan、origin 和 fixture 时还会验证 canonical containment，并拒绝最终路径
为 symlink 的输入。

Compile 与 workflow check 共用有序 producer window。Compile producer 声明 stdout 与 `.bo`；
build producer 声明 link log、生成 C++/object artifact 及其 transfer；run producer 声明 stdout
及其 transfer。只有紧随 producer、路径实际由其产生且 consumer guard 被 producer 覆盖的连续
assertion/comparison 才进入同一 stage；contract、unsupported construct、未组合 action 或不匹配
check 会关闭窗口，后续 producer 会开启新窗口。跨 barrier 的 check 始终保留为
`unbound`/`blocked`，不按 origin 硬编码或猜测。

## Upstream 同步流程

```text
git fetch upstream
git merge upstream/main
pixi run just plans-update
git diff -- rust/tests/plans
pixi run just plans-check
pixi run just plans-audit
```

Importer 输出必须确定性；连续运行两次不得产生 diff。

## 自动门禁

`plans-check`：

1. 860 个 contract `.exp` 与 plan 一一对应；
2. origin hash 和 committed JSON 最新；
3. `index.json` 与分片 plan 对齐；
4. 所有 plan 通过 schema/semantic validation；
5. 无额外、缺失或重复 plan；
6. 所有 fixture 存在且 hash 最新；
7. importer 输出确定性。

`plans-audit`：

1. canonical 文件类型计数发生变化时失败；
2. 不跟随目录符号链接；
3. 新的 Makefile 自定义 recipe 必须分类；
4. 新 `.tcl/.cmd/.pl/.sh` 必须登记 role 与消费者；
5. 新 unsupported 类型不能静默进入 baseline；
6. long-test 开关和 suite group 必须使用路径级 ID，不能依赖重名 stem。

## 分阶段迁移

1. [x] `b1894.exp` 和 `parallel.exp` 验证 ordered sequence plan；
2. [x] plan loader 与 canonical Rust execution kernel；
3. [x] ordinary Bluesim workflow 与 simulation scenario；
4. [x] 基础 compile/golden/error contract；
5. [ ] warning/no-warning 与 object compile failure；
6. [ ] scheduler、Bluetcl、Make-generated fixture、bug/xfail 和 custom cases；
7. [ ] 删除手写 per-origin Rust parity declarations；
8. [ ] 日常运行路径完全移除 Tcl/DejaGNU。

过渡期 Rust case 仅作为 parity oracle，不是新的永久 source of truth。每类 plan 通过冷/热运行
和 alignment 后，立即删除对应手写声明，避免长期维护 `.exp + manifest + plan + Rust case`
四份来源。
