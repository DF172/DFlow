# NOTICE

DraftFlow CAD — 流绘 CAD

Copyright © 2026 DF172 (DraftFlow CAD).

This project is released under the GNU General Public License, version 3.
The full license text is in [LICENSE](LICENSE).

## 上游来源声明

GPL-3 §5 要求衍生作品保留原作品的版权声明，因此本文件保留以下一行，其余内容全部属于本项目：

- Upstream base: Open CAD Studio. The portions originating there remain copyright © Hakan Seven and the Open CAD Studio contributors, licensed under GPL-3.0. This attribution and copyright notice are retained as required by GPL-3 §5; every modification and addition in DraftFlow CAD 流绘 CAD is the work of DF172 (DraftFlow CAD).

本项目在此基础上所做的全部改动（产品名、界面、打包与分发流程、文档、随附 crate 的命名与目录结构）均为本项目自有内容。

## 随附源码 crate

`vendor/` 下的下列 crate 是本项目的源码依赖。各自许可证原文保留在对应 crate 目录中，修改后的文件继续适用其原有许可证。

| 路径 | 许可证 |
| --- | --- |
| `vendor/draftflowcodec` | MPL-2.0 |
| `vendor/draftflowkernel` | MPL-2.0 |
| `vendor/draftflowkernel/crates/draftflowkernel-constraints` | LGPL-2.1-or-later |
| `vendor/draftflowgraph` | MPL-2.0 |

## 第三方 crate

`vendor/iced`、`vendor/iced_aw`、`vendor/winit`、`vendor/cryoglyph` 为 iced-rs 与 rust-windowing 的项目，原样收录，许可证随之。
