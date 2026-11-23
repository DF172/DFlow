# 截图与目视校验

`draftflow_capture` 返回编辑器当前真实显示的画面，并附带把图像像素映射回图纸坐标所需的空间数据。它用于确认几何落在预期位置、比较变更前后状态，以及检查结构 API 无法描述的视觉问题。

## 请求参数

| 参数 | 默认值 | 含义 |
| --- | --- | --- |
| `draftflow_session_id` | 必填 | `draftflow_sessions` 返回的目标会话。 |
| `request_id` | — | 会回显到元数据；截图不是幂等键控操作。 |
| `scope` | `viewport` | `viewport` 只裁剪绘图视口；`window` 包含整个应用窗口。 |
| `max_dimension` | `1600` | 图像最长边，限制在 256–4096。 |
| `delivery` | `inline` | `inline` 在工具结果中内联 base64；`resource` 只返回 `cad://` URI；`both` 两者都返回。 |
| `view` | `current` | `current`、`extents`、`selection` 或 `region`。 |
| `bounds` | — | 世界坐标 `[min_x,min_y,max_x,max_y]`，配合 `view: "region"`。 |
| `focus_handles` | — | 截图前缩放到这些十六进制 handle。 |
| `highlight_handles` | — | 截图前选中这些十六进制 handle。 |
| `annotate` | `false` | 在可见实体上叠加带编号的 Set-of-Marks 标记。 |
| `diff` | `false` | 与当前会话上一次截图比较。 |
| `diff_mode` | `highlight` | `highlight`、`crop` 或 `metadata_only`。 |
| `reset_diff_baseline` | `false` | 丢弃已存基线，本次截图成为新基线。 |
| `tile` | — | `{ "level": 0..6, "x": 列, "y": 行 }`，抓取金字塔中的一块。 |
| `pyramid_manifest` | `false` | 同时生成多级金字塔清单。 |

截图需要 GUI 窗口。窗口若处于最小化会先恢复再渲染；图像在下一帧真正呈现后截取，而不是读取旧的后台缓冲，因此与变更处于同一批次时也能看到变更。结果中的 `waited_for_frame` 表示是否观察到重绘。窗口表面为 0×0（仍是最小化）时会直接失败，而不是读回损坏图像。

## 取景与标注

- `view: "extents"` 适配全部实体；`view: "selection"` 适配当前选择；`view: "region"` 适配 `bounds`。
- `focus_handles` 缩放到指定实体。`highlight_handles` 会先清空选择再选中这些 handle，确保截图高亮的是目标对象。
- `annotate: true` 绘制 Set-of-Marks 编号。编号到 handle、类型、包围盒的映射在 `annotations` 中返回，`annotated` 记录是否绘制了标记。

## 空间基准

每次截图都带 `_spatial`：

- `crs`：`CAD_WCS`
- `unit`：图纸插入单位名称
- `pixel_resolution`：裁剪缩放后的 `[宽, 高]`
- `world_bounds`：图像覆盖的 `{ "min": [x,y], "max": [x,y] }`
- `pixel_to_world_matrix` / `world_to_pixel_matrix`：六元仿射变换
- `camera`：本帧使用的相机
- `target_plane`：视口投影到的平面
- `annotations`：可见实体及其标记与屏幕像素坐标

同样的矩阵、世界包围盒与相机也会在顶层重复为 `pixel_to_world_matrix`、`world_to_pixel_matrix`、`viewport_world_bounds` 与 `camera`；`annotations` 与 `visible_entities` 列出本帧包含的对象。用 `pixel_to_world_matrix` 把可疑像素换算成世界坐标，再查询或测量该点，不要凭肉眼猜测。

## 交付与资源

- `inline` 直接在工具结果中返回 PNG 字节。
- `resource` 只返回 URI，不内联字节；通过 MCP 资源接口获取。
- `both` 同时返回图像与 URI。

已保存的截图可通过以下 URI 访问：

- `cad://session/{session_id}/snapshot/{hash}.png`：某一帧
- `cad://session/{session_id}/snapshot/{hash}.json`：该帧元数据
- `cad://session/{session_id}/snapshot/latest.png`：最近一帧
- `cad://session/{session_id}/viewport.png`：重新抓取的视口
- `cad://session/{session_id}/state.json`：会话状态

结果元数据还包含 `uri`、`latest_uri`、`hash` 与 `bytes`。

## 差异比较

`diff: true` 时，本次截图与当前会话上一次截图比较：

- `diff_mode: "highlight"`：返回完整图像，变化区域高亮，未变化背景压暗。
- `diff_mode: "crop"`：只返回变化区域的裁剪图。
- `diff_mode: "metadata_only"`：图像不变，只填充差异元数据。

比较时 `diff` 对象包含 `is_baseline: false`、`baseline_hash`、`changed`、`change_percentage`、`changed_pixels`、`dirty_pixel_bounds`、`dirty_world_bounds`、`patch_resolution` 与 `mode`；基线截图只含 `is_baseline: true`、`changed: false`、`change_percentage`、`changed_pixels`、`message` 与 `mode`。若比较结果没有变化，媒体会被省略，只返回元数据，便于低成本轮询。会话的第一次截图（或执行 `reset_diff_baseline` 之后）是基线，总会返回图像。

## 分块与金字塔

`tile` 抓取多级金字塔中的一个单元：0 级是总览，1 级是 2×2 网格，2 级是 4×4 网格，依此类推，最高 6 级。响应元数据增加 `pyramid` 对象，包含 `level`、`x`、`y`、`tile_world_bounds`、`tile_uri` 与 `manifest_uri`；分块默认 512 px。`pyramid_manifest: true` 会返回完整清单（层级、网格尺寸、每块的世界范围与资源 URI），位置为 `cad://session/{session_id}/pyramid/manifest.json`。单块资源位于 `cad://session/{session_id}/tile/{level}/{x}/{y}.png`，加 `.json` 可取得元数据。

## 校验闭环

1. 抓取基线（会话已有基线时加 `reset_diff_baseline`）。
2. 通过结构 API 应用变更。
3. 再次截图并设置 `diff: true`，读取 `dirty_world_bounds`。
4. 在变化的世界包围盒内 `query` 或 `measure`，确认变更正确。
5. 保存前抓取最终状态，重新打开保存的文件后再抓一次。

精确数值优先用结构校验（`query`、`measure`、`audit`）；截图用于发现数值看不到的问题：裁切、重叠、图层可见性错误，或几何存在但在画面之外。
