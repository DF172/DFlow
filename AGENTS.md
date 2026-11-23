# AGENTS.md — DraftFlow CAD 工程约定

本仓库的构建是重型的（2000+ 源文件，全量 release 约 15 分钟）。任何自动化智能体在本仓库里干活，**必须默认走下面的极速工作流**，不要直接敲 `cargo build --release`。

## 1. 默认命令（优先用 `just`，别用裸 cargo）

| 想干什么 | 用这个 | 不要用 | 说明 |
| --- | --- | --- | --- |
| 查编译错误 | `just check` | `cargo check` | 无改动约 2–3 s，改一处约 17 s |
| 跑测试 | `just test` | `cargo test` | nextest 全量约 2 分钟（2720 条）+ doctest |
| 跑单条 / 过滤 | `cargo nextest run -E 'test(名字)'` | `cargo test 名字` | 过滤表达式见 nextest 文档 |
| 起界面看效果 | `just run` | `cargo run --release` | fast-release 档，增量约 5 分钟 |
| 出正式 exe | `just ship` | — | release 档（fat LTO + strip），单并发，只在发版/交付时用 |
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
8. **已知天坑：release 全量编译会撞 Windows 命令行上限，sccache 报 `os error 206`**（2026-11-19 实测，同命令连续复现 2 次）。主 crate 的 rustc 命令行约 55 143 字符，超过 Windows `CreateProcess` 的 32 767 上限，sccache 转发 rustc 失败，日志尾巴是 `sccache: caused by: 文件名或扩展名太长 (os error 206)` 与 `exit code 0xfffffffe`、`CARGO_EXIT=101`。`cargo check`、dev、fast-release 都不受影响（命令行更短）。
   - 对策已固化在 `just ship`：recipe 里 `RUSTC_WRAPPER= CARGO_BUILD_JOBS=1 cargo build --release`，显式绕过 sccache 并单并发。
   - 手动发版同样要加前缀：`RUSTC_WRAPPER= cargo build --release`。
   - 代价与纪律：wrapper 变更会触发全树重编（实测 22m49s），所以 release 档**永远不加回 sccache**；不要在 release 里来回切换 wrapper。
   - 若将来 sccache 升级并支持响应文件转发，可重测后撤销本条，先测再改。

## 4. 测试门槛

- 改动后至少 `just check` 全绿。
- 提测/部署前 `just test` 全量绿（`2720 tests run, 0 failed` 为基线，会随功能增加）。
- `fps_probe_2000_scroll` 是 wall-clock 探针，已在 `.config/nextest.toml` 里独占全部核心；**不要放宽它的 20 ms 阈值**来让它变绿。
- 只许加强测试，不许弱化或禁用断言。

## 5. 目录与缓存

- 唯一工作树：本目录。
- `CARGO_TARGET_DIR=D:\cargo-target`（不写进仓库树）。
- rust-analyzer 用 `D:/cargo-target-ra`（`.vscode/settings.json`），避免和命令行抢锁。
- sccache 缓存在 `D:/sccache`，上限 10 GiB。
- Defender 白名单脚本：`scripts/defender-exclusions.ps1`（需管理员）。

## 6. 工具链强制加载（2026-11-21 起）

- 任何构建、测试、运行、发版前先执行 `just preflight`；`check/test/run/ship/perf` 已内置该依赖，不要绕过。
- `check`、`test` 与 `ship` 固定 `CARGO_BUILD_JOBS=1`：8 并发下 rustc 在本机多次以 `0xC0000409` 崩溃（2026-11-21 实测，全量清理后仍复现），单并发稳定通过。
- 标准运行：`just run` 固定 `RUST_BACKTRACE=1`、`RUST_LOG`（默认 info），并把完整输出同时写入 `D:\Temp\draftflow-run.log`。
- 标准性能门：`just perf`，只跑 `fps_probe_2000_scroll`，20 ms 阈值不得放宽。
- 工具缺失或版本冲突必须记录并说明影响，不得静默跳过；必需项缺失时 `preflight` 直接失败。
- 同一时间只保留一个 `DraftFlowCAD.exe` GUI 实例；MCP 附着现有实例，重复启动会因显存不足崩溃（0xc0000409）。
- 调试用 CodeLLDB（`vadimcn.vscode-lldb`），语言服务用 rust-analyzer；两者由 `preflight` 检查。

## 7. 验证流程（按门执行，不重复）

- 常规改动：`just check` → `just test`（全量，已含 `fps_probe_2000_scroll`）→ 需要看界面时 `just run`。已被上一级覆盖的用例不再单列。
- `just perf` 只在改渲染/性能相关代码时单独跑，不进入常规验证清单。
- 界面与自动化验证合并为一次进程会话：启动一次 GUI → 截图或人工确认 → MCP 握手 + 一次真实调用 → 关闭；同一二进制不重复握手。
- 发版：`just ship`（已固定单并发）→ 校验 exe 版本。发版前必须确认 rust-analyzer 已退出（其私有提交约 5 GB）且 commit 余量充足，否则会 `rustc-LLVM ERROR: out of memory`。
- 安装包：`D:\app\innosetup-7.1.1-x64\ISCC.exe packaging\windows\DraftFlowCAD.iss`，产物 `dist\DraftFlowCAD-<版本>-x64-setup.exe`；安装/卸载实测需另行确认，不默认执行。
- 失败重试最多一次，且必须变更条件并在报告中写明；禁止原样重跑同一命令。
