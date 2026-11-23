# 补齐 docs/automation 四篇空文档

## Context（为什么做）

`docs/automation/` 下 4 个文件当前只有 1 字节（仅换行），但它们是：

1. README 第 121–124 行公开链接的项目文档；
2. `src/mcp.rs` 中 `GUIDES` 数组通过 `include_str!` 编译进二进制的 MCP 指南（`cad://guides/{name}` 与 `draftflow_read op guide`）。

空文档会让 MCP 客户端拿到空字符串，等于指南缺失。本次只补内容，不改协议、不改代码行为。

## 事实依据（已核对代码）

- 四通道：`--serve`（stdio 行式 JSON，`src/app/automation.rs:100`）、`--serve --port N`（TCP 127.0.0.1，单客户端、15s 空闲超时、token，`automation.rs:130-156`）、`--http N`（REST，Bearer token，`src/rest.rs:1-27`，GUI 模式下由 `control::http_bridge` 承载）、`--mcp`（MCP over stdio，转发到 GUI 控制桥，`src/mcp.rs:1-45`）。
- 共享分发：`DraftFlowCAD::automation_op` → `automation_op_inner`（legacy ops）或 `control_request`（`protocol` 为数字时的协议 1 信封，`automation.rs:727-757`、`control/mod.rs:421-466`）。
- 幂等：`request_id` 必填、≤128 字节、同 id 同 payload 返回缓存、不同 payload 返回 `request_id_reused`（`control/mod.rs:584-633`）；`operation` 轮询；`events` 游标（`control/mod.rs:482-507`）。
- `entities_create`：先全量校验再建实体，自动建缺失图层，单次 undo（`control/entities.rs:539-591`）；支持 LINE/CIRCLE/ARC/SPLINE/LWPOLYLINE/POLYLINE/POINT/TEXT/MTEXT/INSERT/SOLID/HATCH（`entities.rs:96-147`）。
- `save`：`allow_lossy` 门、`target_format`/扩展名冲突检查（`control/mod.rs:1001-1048`、`automation.rs:500-536`）。
- `save_verified`：绝对路径、`.dwg/.dxf`、`overwrite`、预保存 `audit`、lossy 门、SHA-256、ASCII DXF handle 图审计、重开、版本一致、语义清单一致（`automation.rs:1565-1699`）。
- `capture`：需要 GUI；`scope` 默认 viewport，`max_dimension` 256–4096 默认 1600；等待下一帧（`waited_for_frame`）；`view`/`bounds`/`focus_handles`/`highlight_handles`/`annotate`；返回 `_spatial`（CAD_WCS、unit、pixel/world 矩阵、世界包围盒、相机、可见实体）（`control/mod.rs:1054-1119`、`1323-1489`）。
- MCP `draftflow_capture` 参数：`scope`、`max_dimension`、`delivery`、`diff`、`diff_mode`、`reset_diff_baseline`、`view`、`bounds`、`focus_handles`、`highlight_handles`、`annotate`、`tile{level,x,y}`、`pyramid_manifest`（`mcp.rs:2577-2602`）。
- 资源 URI：`cad://session/{id}/snapshot/{hash}.png|.json`、`snapshot/latest.png`、`viewport.png`、`state.json`、`pyramid/manifest.json`、`tile/{level}/{x}/{y}.png|.json`（`mcp.rs:1804-2078`、`2401-2435`）。
- `batch`：MCP 适配器侧顺序执行（`mcp.rs:1209-1440`），1–64 步，步内禁止 `request_id`，作用域 `BATCH_STEP_OPS`（`mcp.rs:117-158`）；`bindings` 语法为 `{"<目标字段>":{"step":<索引>,"path":"<JSON Pointer>"}}`，源步必须 completed 且 ok（`mcp.rs:1616-1640`）；失败/取消立即终止，`waiting_input` 终止（后续步为 `input`/`cancel` 除外）；`response_detail:"summary"` 压缩输出，`draftflow_read op batch_result` 只读取保留结果（每连接 64 条）。

## 改动文件（全部为中文正文）

### 1. `docs/automation/guides/architecture.md`
标题“建筑矢量化建模”。内容：
- 分层约束：entities→scene→app→ui；app 是唯一变更入口；`mcp.rs` 不含几何，仅协议适配。
- 四通道与共享语义（stdio / TCP / HTTP / MCP；同一套校验、undo、幂等、request_id 规则）。
- 协议 1 信封字段、轮询、事件。
- 矢量化建模原则：用原生实体而非线段堆；LWPOLYLINE（`closed`/`constant_width`）表达墙与拓扑；ARC/CIRCLE、TEXT/MTEXT、INSERT、HATCH；按专业命名图层，`entities_create` 自动建层。
- 矢量化流程：先从 `header`/`measure` 定单位与比例 → 精确坐标 → 一次 `entities_create`（单 undo）→ `query`/`measure`/`audit` 校验 → `capture` 目视 → `save_verified`。
- 禁止：近似内核几何、跳过校验、逐实体零散写入。

### 2. `docs/automation/guides/workflow.md`
标题“CAD 自动化工作流”。内容：
- 连接：`DraftFlowCAD --mcp` → `draftflow_sessions` → 读 `state`/`capabilities`/`record_schema`/`commands`。
- 文档生命周期：`new`（可 `template`）、`open`（路径或 `data_base64`+`name`）、`activate`/`switch_document`、`close`。
- 观察：`query`（detail/filters）、`records`、`text_search`、`measure`、`layers`、`header`、`audit`、`events`。
- 编辑：`run` 一次性命令；`start`/`input`/`cancel` 交互状态机（`waiting_input`）；`entities_create`/`delete`/`transform`；`set_properties`、`block_define`、`xdata_set`、`view_focus`、`select`。
- 请求纪律：唯一 `request_id`；超时用 `operation` 查询；绝不换新 id 重放变更；`document_id`/`revision` 守卫。
- 保存：`save`（save-as、lossy 门）与 `save_verified`（绝对路径、`overwrite`、预审计、SHA-256、DXF handle 图、重开、版本、语义清单）及返回字段；`--export` 一次性转换。
- 一段 `new → entities_create → capture → save_verified` 的 JSON 序列示例。
- 说明：同一 ops 也走 HTTP/stdio/TCP；`batch` 仅 MCP 适配器提供。

### 3. `docs/automation/guides/capture.md`
标题“截图与目视验证”。内容：
- `draftflow_capture` 参数表（默认值与范围照代码）。
- 捕获语义：仅 GUI；默认裁剪绘图视口；最小化会先恢复；停到下一帧再截，`waited_for_frame` 字段；0×0 面报错。
- 取景与标注：`view`/`bounds`/`focus_handles`、`highlight_handles` 会重选、`annotate` 叠加 Set-of-Marks 编号（`annotated`/`annotations`）。
- 空间基准：`_spatial`（CAD_WCS、unit、pixel_resolution、world_bounds、pixel_to_world/world_to_pixel 6 元仿射、camera、target_plane、annotations），用于像素↔世界映射与位置校验。
- 交付：`inline`/`resource`/`both`；快照与 `viewport.png`、`state.json` 资源 URI；尺寸上限。
- 差异：`diff` + `diff_mode`（highlight/crop/metadata_only）+ `reset_diff_baseline`；元数据字段（is_baseline、baseline_hash、changed、change_percentage、changed_pixels、dirty_pixel_bounds、dirty_world_bounds、patch_resolution、mode）；未变化时省略媒体。
- 分块：`tile{level,x,y}`、`pyramid_manifest`、manifest 与 tile URI、默认 512px。
- 目视验证闭环：基线 → 变更 → diff 看 dirty 世界包围盒 → 修正 → 再捕获；保存前后各捕获一次。

### 4. `docs/automation/batch-bindings.md`
标题“依赖批次步骤”。内容：
- 定位：`batch` 是 MCP `draftflow_execute` 的适配器侧编排（非 GUI 服务端 op），1–64 步顺序执行。
- 步骤约束：禁止自带 `request_id`（适配器按批次 id+索引生成）；op 必须在允许表内（列出代表项）。
- `bindings` 语法与示例（`entities_create` 的 `/handles` 绑定给 `set_properties.handles` 或 `view_focus.handles`）；源步必须成功完成，路径必须是 RFC 6901 JSON Pointer；绑定失败整批 `invalid_batch_binding`。
- 执行语义：逐步执行；任一步 failed/cancelled 立即终止并保留已完成结果；`waiting_input` 终止（除非下一步是 `input`/`cancel`）；同一 `request_id` 重发可续跑，不同 payload 拒绝。
- 响应与摘要：`completed_steps`/`total_steps`/`next_step`/`results`/`changes`/`state`；`response_detail:"summary"` 输出 `failed_steps`/`change_count`/`attention`/`details`；`batch_result` 只读、仅本连接、保留 64 条。
- 反模式：拆成多个批次重复变更、用新 `request_id` 重放、在步骤里手写 `request_id`。

## 验证

1. 内容一致性：逐条对照上列代码位置复核字段名、默认值、错误码。
2. `just check`（编辑 `include_str!` 会触发 `mcp.rs` 重编，必须全绿；AGENTS 已固定 `CARGO_BUILD_JOBS=1`）。
3. 定向测试：
   - `cargo nextest run -E 'test(guides_are_discoverable_without_a_gui_session)'`
   - `cargo nextest run -E 'test(tool_schema_digest_is_pinned)'`（确认文档内容不进入工具 schema 摘要）
4. 记录退出码；文档改动不涉及样式棘轮、20ms 阈值或依赖。

## 风险与回滚

- 风险：文档示例与实现漂移。缓解：所有示例字段均来自上列代码位置；不写未验证的端点/错误码。
- 回滚：四文件均为纯文档，`git checkout -- docs/automation` 即可；不影响二进制协议（仅 `include_str!` 内容变化）。

## 未验证项

- 未启动 GUI 实机跑 `draftflow_capture`/`save_verified`（需单实例 GUI，按 AGENTS 约束不在本轮启动）；文档只描述代码已实现的行为。
- REST 具体路由表未逐条写入文档，只在架构篇说明通道与鉴权，避免与 `rest.rs` 路由漂移。
