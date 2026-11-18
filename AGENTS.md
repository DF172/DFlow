# AGENTS.md — DraftFlow CAD 工程约定

本仓库的构建是重型的（2000+ 源文件，全量 release 约 15 分钟）。任何自动化智能体在本仓库里干活，**必须默认走下面的极速工作流**，不要直接敲 `cargo build --release`。

## 1. 默认命令（优先用 `just`，别用裸 cargo）

| 想干什么 | 用这个 | 不要用 | 说明 |
| --- | --- | --- | --- |
| 查编译错误 | `just check` | `cargo check` | 无改动约 2–3 s，改一处约 17 s |
| 跑测试 | `just test` | `cargo test` | nextest 全量约 2 分钟（2641 条）+ doctest |
| 跑单条 / 过滤 | `cargo nextest run -E 'test(名字)'` | `cargo test 名字` | 过滤表达式见 nextest 文档 |
| 起界面看效果 | `just run` | `cargo run --release` | fast-release 档，增量约 5 分钟 |
| 出正式 exe | `just ship` | — | release 档（fat LTO + strip），约 15 分钟，只在发版/交付时用 |
| 清旧产物 | `just sweep` | `rm -rf target` | 清 7 天前的中间产物 |

## 2. Profile 三档

- **dev**：opt0 + debug1 + incremental；依赖包 opt2（`[profile.dev.package."*"]`），调试不卡。
- **fast-release**：继承 release 的 strip/panic=abort，但 opt2 / `lto=false` / `codegen-units=16`。**日常验证可运行 exe 一律用它**，产物不是发版件。
- **release**：opt3 / `lto="fat"` / `codegen-units=1` / strip / panic=abort。唯一发版档。

## 3. 硬约束（都是实测踩坑换来的）

1. **`jobs = 8`，不要调到 16**。16 GB 内存机器上 16 并发会让 `rustc-LLVM ERROR: out of memory`（0xc0000409）。8 已经能吃满 16 逻辑核，因为每个 rustc 内部还并行 codegen。
2. **sccache 必须写在 `.cargo/config.toml` 的 `build.rustc-wrapper`**。写成 `[env] RUSTC_WRAPPER` 会静默失效（cargo 读 wrapper 早于注入 `[env]`），`sccache --show-stats` 恒为 0。
3. **dev 增量编译不可缓存**。日常改代码看不到 sccache 命中是正常的；它只在冷全量、换依赖、换 target 时生效。
4. **后台构建必须把退出码写进日志**。`cargo build | tee log` 拿到的是 tee 的退出码；用 `cargo build > log 2>&1; echo BUILD_EXIT=$? >> log`。
5. **构建中不要改源码**。中途编辑会让本轮产物作废，增量重建仍需整轮时间。
6. **日志可能含 NUL 字节**，读之前先 `tr -d '\000'` 或 `grep -a`。
7. **文件锁探测禁用 `: > file`**（会把文件清空）。用 `mv` 改名即可绕过运行中 exe 的占用。

## 4. 测试门槛

- 改动后至少 `just check` 全绿。
- 提测/部署前 `just test` 全量绿（`2641 tests run, 0 failed` 为基线，会随功能增加）。
- `fps_probe_2000_scroll` 是 wall-clock 探针，已在 `.config/nextest.toml` 里独占全部核心；**不要放宽它的 20 ms 阈值**来让它变绿。
- 只许加强测试，不许弱化或禁用断言。

## 5. 目录与缓存

- 唯一工作树：本目录。
- `CARGO_TARGET_DIR=D:\cargo-target`（不写进仓库树）。
- rust-analyzer 用 `D:/cargo-target-ra`（`.vscode/settings.json`），避免和命令行抢锁。
- sccache 缓存在 `D:/sccache`，上限 10 GiB。
- Defender 白名单脚本：`scripts/defender-exclusions.ps1`（需管理员）。
