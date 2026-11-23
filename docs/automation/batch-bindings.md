# 有依赖的批处理步骤

`batch` 是 MCP `draftflow_execute` 提供的一种编排能力：由适配器按顺序执行多个单步操作，并把前面步骤的结果绑定到后面步骤的字段里。它不是编辑器服务端的操作，因此只通过 MCP 工具使用。

## 基本约束

- 一次批处理包含 1–64 个步骤，按数组顺序逐个执行。
- 每个步骤是一个 JSON 对象，必须有 `op`，且 `op` 必须属于批处理允许的操作集：
  `new`、`open`、`activate`、`switch_document`、`run`、`start`、`input`、`cancel`、`undo`、`redo`、`select`、`property`、`grip_drag`、`click`、`set_properties`、`action`、`embed_image`、`wblock`、`plot`、`entities_create`、`entities_delete`、`entities_transform`、`text_replace`、`block_define`、`block_delete`、`xdata_set`、`view_focus`、`entities_copy_to`、`group_create`、`selection_set_save`、`selection_set_load`、`user_select`、`getpoint`、`close`、`sysvar`、`layout_create`、`page_setup_set`、`file_identity`、`save`、`stop`。
- 步骤内**禁止**自带 `request_id`。适配器会按“批次 request_id + 步骤序号”派生稳定 id，避免客户端手写幂等键造成冲突。

## bindings 语法

在需要引用前序结果的步骤里加一个 `bindings` 对象：

```json
{
  "op": "entities_delete",
  "handles": [],
  "bindings": {
    "handles": { "step": 0, "path": "/handles" }
  }
}
```

- 键是当前步骤要填充的目标字段名。
- `step` 是来源步骤在 `steps` 数组中的下标（从 0 开始）。
- `path` 是 RFC 6901 JSON Pointer，指向来源步骤响应中的值。

解析规则：

- 来源步骤必须已经执行完成，且 `status` 为 `completed`、`ok` 不为 `false`；否则整个批次失败。
- 指针在来源响应中不存在时，批次失败。
- 上述失败统一以 `invalid_batch_binding` 结束批次，失败步骤会记录在结果中，后续步骤不再执行。
- 绑定值会覆盖目标字段的占位值，因此占位值只需满足结构校验，不需要真实数据。

## 完整示例

创建一条线，然后把返回的 handle 直接用于删除与视图聚焦：

```json
{
  "op": "batch",
  "request_id": "line-then-cleanup",
  "steps": [
    {
      "op": "entities_create",
      "entities": [
        {"type": "LINE", "start": [0, 0], "end": [5000, 0], "layer": "A-WALL"}
      ]
    },
    {
      "op": "view_focus",
      "handles": [],
      "bindings": { "handles": { "step": 0, "path": "/handles" } }
    },
    {
      "op": "entities_delete",
      "handles": [],
      "bindings": { "handles": { "step": 0, "path": "/handles" } }
    }
  ]
}
```

`entities_create` 的响应包含 `handles`、`created` 与 `layers_created`，所以 `/handles` 是常用绑定路径。其他步骤的结果也可绑定，例如运行命令后的 `state` 字段。

## 执行语义

- 逐步执行。每一步的响应会记录到 `results`，变更记录汇总到 `changes`。
- 任一步 `ok` 为 `false`，或 `status` 为 `failed`/`cancelled`，批次立即终止；该步结果保留，后续步骤不执行。
- 步骤返回 `waiting_input` 时，若下一步是 `input` 或 `cancel`，批次继续；否则批次以 `waiting_input` 终止，等待客户端补充输入后再继续。
- 步骤返回 `accepted`/`running` 时，批次返回 `status: "running"` 并带上 `active_step`。用**完全相同**的批次请求（同一 `request_id` 与同一载荷）重新调用即可续跑；载荷不同会被拒绝。
- 取消会放弃整个批次，因为继续执行会重新轮询已撤销的步骤。

## 响应与摘要

批次结果的字段包括：

- `completed_steps`、`total_steps`、`next_step`
- `results`：每个已完成步骤的响应
- `changes`：汇总的变更记录，并标注来源 `step`
- `state`：最近一次观察到的编辑器状态
- `active_step`：仍在执行的步骤（`running` 时）

`draftflow_execute` 的 `response_detail` 默认为 `compact`。设为 `summary` 时，批次响应会省略 `results` 与 `changes`，改为返回 `change_count`、`failed_steps`、`attention`（失败步骤与需要注意的告警）以及 `details`；后续可用 `draftflow_read` 的 `batch_result`（参数 `request_id`）读取保留的完整结果。

`batch_result` 是只读路径：它不会连接、轮询、恢复或重建批次。保留结果仅存在于发起批次的 MCP 连接中，每个会话最多保留 64 条；结果缺失或过期时应重新执行必要操作，而不是重放变更。

## 反模式

- 把本可一次完成的变更拆成多个批次，导致撤销步碎片化或中途失败留下半成品。
- 超时后换一个新的 `request_id` 重放批次；这会重复创建几何。
- 在步骤里手写 `request_id`；适配器会拒绝这种请求。
- 用 `bindings` 引用尚未执行或已失败的步骤。
- 用 `batch_result` 尝试重新执行；它只用于读取。
