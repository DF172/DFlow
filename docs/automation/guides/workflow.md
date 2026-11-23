# CAD 自动化工作流

典型自动化会话的流程固定：连接、打开或新建文档、观察、修改、校验、保存，并证明保存结果正确。下列操作均为协议 1 请求，MCP 工具只是对它们的包装。

## 连接

先在编辑器侧启动一次（同一时间只允许一个 GUI 实例）：

```bash
DraftFlowCAD --mcp
```

客户端侧：

1. `draftflow_sessions` 列出活动会话。每个描述符包含 `session_id`、桥接 `port` 和 `token`。
2. 之后每次工具调用都传入 `draftflow_session_id`。
3. 先读 `state`（`draftflow_read` 的 `state` 操作）。面对不熟悉的图纸，再读 `capabilities`、`record_schema` 与 `commands`；可用 `{"op":"commands","name":"LINE"}` 查询单个命令的批处理语法和交互式输入指引。

同样的操作也可通过 `--serve`（stdio）、`--serve --port`（TCP）与 `--http`（带 Bearer 令牌的 REST）访问。`capabilities` 说明当前会话启用了什么，`tools` 返回 MCP 工具定义。

## 打开或新建文档

- `new` 新建空白图纸。`{"op":"new","template":"acad.dwt"}` 会应用模板，并在实体摘要之外返回 `template` 与 `purged`。
- `open` 按 `path` 加载文件，或以内联字节加载：`{"op":"open","name":"plan.dwg","data_base64":"…"}`。名称会缩减为文件名，用于标签页。
- `activate` / `switch_document` 在已打开的文档间切换；`close` 关闭当前文档。

响应包含 `document_id` 与文档元数据。务必保留该 id：它是后续变更的守卫，可防止请求落到错误的标签页。

## 观察

- `query`：按 `handles`、`type`、`layer`、`bounds`、`near` 或 `contains_point` 查询实体。`detail` 可选 `summary`、`geometry`（默认）、`full`；`fields` 只投影指定属性。
- `records` / `record_schema`：查询表及其可写路径。`records` 支持 RFC 6901 路径与 `where` 过滤（`eq`、`ne`、`lt`、`lte`、`gt`、`gte`、`contains`、`starts_with`、`ends_with`、`in`、`exists`、`not_exists`）。
- `text_search` / `text_audit`：查找文本、模拟替换。
- `measure`：由内核计算长度、面积、体积与质量属性。
- `layers`、`header`、`history`、`audit`、`events`：结构与变更流。

## 修改

- `run` 执行一条命令字符串，使用与命令行相同的分发器：`{"op":"run","cmd":"LINE 0,0 5000,0"}`。点坐标用 `x,y` 或 `x,y,z`；选项使用其标记。
- `start` / `input` / `cancel` 分步驱动交互式命令。`start` 之后，按 `state.command.accepts`、可选 `options` 与 `input_example` 继续输入。`waiting_input` 表示还需下一步输入；`completed` 是最终状态；`failed` 会给出原因。
- `entities_create` 以一次原子提交创建类型化实体。所有定义先校验，缺失图层自动创建，整批只占一个撤销步。响应返回 `handles`、`created` 与 `layers_created`。
- `entities_delete`、`entities_transform`、`text_replace`、`set_properties`、`block_define`、`block_delete`、`xdata_set`、`group_create`、`view_focus`、`select` 均基于 `query`/`records` 返回的 handle 操作。

### 请求纪律

- 每个变更都要有唯一 `request_id`（≤128 字节）。只有重试**完全相同**的载荷时才能复用它；载荷变化会返回 `request_id_reused`。
- 响应为 `accepted`/`running` 时，用 `{"op":"operation","request_id":"…"}` 轮询。超时后不要换新 id 重放；那可能重复创建几何。
- 传入 `document_id`，并在客户端持有当前值时传入 `revision` / `geometry_revision` / `camera_revision`。过期值会被拒绝并返回当前状态，而不是被应用。
- `busy` 表示仍有操作未完成。应等待，或对 `user_select`/`getpoint` 这类交互请求发送 `cancel`。

## 保存与验证

`save` 写入当前文档（也可另存为其他路径）：

```json
{"protocol":1,"request_id":"save-1","op":"save","path":"D:\\out\\plan.dwg"}
```

- `target_format`（`dwg`/`dxf`）与 `target_version` 选择输出格式与版本；`target_format` 与路径扩展名冲突时直接拒绝。
- 若转换会丢弃不支持的记录，请求失败并返回 `lossy_conversion_not_acknowledged` 与 `dropped_on_save`。确认接受损失后，才用 `"allow_lossy": true` 重发。

`save_verified` 是交付路径。它要求绝对路径的 `.dwg`/`.dxf`，目标文件已存在时必须显式 `overwrite: true`，然后依次：

1. 审计源结构；
2. 无 `allow_lossy` 时拒绝有损转换；
3. 按指定版本写入文件；
4. 返回字节的 SHA-256；
5. 目标为 DXF 时审计 ASCII DXF 的 handle 图（重复、悬空、孤立 handle）；
6. 重新打开文件并检查 CAD 版本；
7. 比较语义实体清单（按类型与图层统计，忽略自动生成的标注显示块）。

成功结果包含 `verified: true`、`sha256`、`bytes`、`target_format`、`target_version`、`dropped_on_save`、`manifest`、`audit` 与 `dxf_structure`。失败时文件会保留用于诊断，并给出具体失败检查：`audit_failed`、`lossy_conversion_not_acknowledged`、`dxf_audit_failed`、`invalid_dxf_handle_graph`、`reopen_failed`、`version_mismatch`、`semantic_mismatch`。

不建立会话的一次性转换可走 CLI：

```bash
DraftFlowCAD --export input.dwg output.dxf
DraftFlowCAD --export input-r12.dxf output.dwg --target-version R14
```

## 端到端示例

一个最小 MCP 会话：创建一段墙、检查、抓取视口，再带验证保存。

```json
{"op":"new"}
{"op":"entities_create","entities":[
  {"type":"LWPOLYLINE","vertices":[[0,0],[5000,0],[5000,3000],[0,3000]],
   "closed":true,"constant_width":200,"layer":"A-WALL"}
]}
{"op":"query","type":"LWPOLYLINE","detail":"geometry"}
{"op":"audit"}
```

随后用 `draftflow_capture`，参数为 `scope: "viewport"`、`view: "extents"`，最后：

```json
{"op":"save_verified","path":"D:\\out\\wall.dwg","overwrite":true}
```

矢量化平面图前先读 `architecture`；目视校验前读 `capture`；编排有依赖的步骤前读 `batch-bindings`。
