# 建筑图纸矢量化

本文说明 DraftFlow 自动化的分层结构，以及如何把建筑输入（扫描图、PDF、栅格图）转换成原生 CAD 几何，而不是与数据模型对抗。

## 分层与职责

编辑器按层划分，自动化必须遵守同样的边界：

- `entities`：实体类型及其几何数据。
- `scene`：图形数据库、拾取、捕捉与修改。
- `app`：唯一允许修改应用状态的地方。无论是功能区按钮、命令行还是 JSON 请求，最终都进入这里。
- `ui`：只负责渲染与输入，不直接修改实体。

自动化模块是协议适配层。`src/mcp.rs` 不包含任何几何逻辑：它把 MCP 调用翻译成协议请求并转发给正在运行的编辑器。所有创建或修改图形记录的操作都走与 GUI 相同的 `app` 处理函数，因此撤销、校验与文档修订号的表现完全一致。

## 一个分发器，四种传输通道

操作分发器只有一个：`DraftFlowCAD::automation_op`，可通过四种方式访问：

| 通道 | 启动方式 | 说明 |
| --- | --- | --- |
| stdio | `DraftFlowCAD --serve` | 每行一个 JSON 对象输入，每行一个响应输出；首先打印一行 `ready`。 |
| TCP | `DraftFlowCAD --serve --port 4242` | 绑定 `127.0.0.1`，同一时间服务一个客户端，空闲 15 秒断开；会话在重连后保留。 |
| HTTP | `DraftFlowCAD --http 8090` | `/api/v1` 下的资源式 JSON 端点；每个请求需 `Authorization: Bearer <token>`，令牌在启动时打印；仅回环地址，`Connection: close`。 |
| MCP | `DraftFlowCAD --mcp` | 面向 AI 客户端的 MCP over stdio；请求转发给经过认证的 GUI 控制桥。 |

前三种通道使用同一套行式协议。请求中带数字 `protocol` 字段时走协议 1 控制信封；否则走紧凑的旧版操作集（`new`、`open`、`run`、`entities`、`save` 等）。HTTP 的通用 `POST /api/v1/{op}` 只在 `ENVELOPE_OPS` 列出的变更操作上使用协议 1 信封，其余仍走旧版；MCP 工具请求始终使用协议 1 信封，并由适配器补齐 `request_id`。信封内的校验、撤销分组与幂等规则共享，因此同一请求无论从哪个通道进入，行为都一致。

## 协议 1 信封

变更请求形如：

```json
{"protocol":1,"request_id":"draw-1","op":"entities_create","entities":[]}
```

- `protocol` 出现时必须为 `1`。
- 变更操作必须带 `request_id`，全局唯一，最长 128 字节。同 id 同载荷返回缓存结果；载荷变化则返回 `request_id_reused`。此时应生成新 id，而不是修改原请求。
- `document_id` 与修订字段（`revision`、`geometry_revision`、`camera_revision`）是守卫。值过期时请求被拒绝，避免把变更应用到错误的文档或错误的状态。
- 部分变更异步执行。响应 `status` 为 `accepted` 或 `running` 时，用 `{"op":"operation","request_id":"…"}` 轮询直到完成。超时绝不换新 id 重放变更。
- `{"op":"events","after":<cursor>}` 输出变更事件；`resync` 表示客户端应重新加载状态。

`capabilities` 返回启用的操作分组，`record_schema` 描述所有可写记录路径。

## 用原生实体表达建筑模型

不要把平面图拆成一堆互不相干的线段。应使用能承载语义的实体：

- `LWPOLYLINE`（或 `POLYLINE`）用于墙段、边界和任何连通拓扑。闭合环设 `closed: true`；墙厚用 `constant_width`。一条多段线保留连接关系，而独立线段会丢失。
- `ARC` 与 `CIRCLE` 用于弧形墙、圆柱和门扇开启弧。请求中的角度单位为度，且必须是有限值。
- `TEXT` 与 `MTEXT` 用于房间名、标记和注释。必须给出明确的 `position`、`height` 和已存在的 `style`。
- `INSERT` 用于重复构件（门、窗、家具）。先用 `block_define` 定义一次块，再放置引用；不要把同一几何复制多份。
- `HATCH` 用于填充区域（墙体涂黑、场地）。实体填充需要闭合的 `boundary`。
- `SPLINE`、`POINT`、`SOLID` 仅在源数据确实需要时使用。

图层按专业命名（`A-WALL`、`A-DOOR`、`A-ANNO` 等）。`entities_create` 会自动创建缺失图层，且新建图层与实体处于同一个撤销步中。

## 矢量化流程

1. 确定单位与比例。读取 `header` 与 `measure`，不要假设单位是毫米。先把每个像素点或 PDF 点换算成世界坐标，再构造请求。
2. 对齐到精确几何。使用 `snap`/`query`/内核测量，而不是估算坐标。共享端点必须是完全相同的数值，而不是近似相等，否则多段线和填充无法正确闭合。
3. 一次构造类型化批次。把全部实体放进一个 `entities_create` 的数组中。所有定义在添加第一条实体前完成校验，提交只产生一个撤销步，响应返回新建的 `handles` 与 `layers_created`。
4. 结构校验。用 `query`（`detail: "geometry"`）、`measure` 和 `audit` 确认数量、包围盒、长度与面积。`audit` 会报告重复或悬空的 handle，交付前必须修复。
5. 目视校验。用 `draftflow_capture` 抓取视口并与源图比对。借助 `_spatial` 映射把可疑像素换算回世界坐标，再查询该点。
6. 带证明地保存。`save_verified` 写入绝对路径的 `.dwg`/`.dxf`，保存前审计，计算字节哈希，重新打开文件并比较语义清单。

## 应避免的做法

- 明明有 `ARC`/`CIRCLE`/`SPLINE`，却用大量短线近似曲线。
- 一条请求只建一个实体；这会产生大量撤销步，且后续请求失败时会留下半矢量化图纸。
- 客户端自行计算几何，而内核已经提供 `measure`、`snap` 和交点计算。
- 跳过审计，写出一个从未重新打开验证的文件。
- 超时后复用同一个 `request_id` 发送不同载荷。
