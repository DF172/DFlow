# NOTICE

DraftFlow CAD — 流绘 CAD ("this project") is a modified fork of
**Open CAD Studio**, an open-source CAD application distributed under the
GNU General Public License, version 3.

- Upstream project: https://github.com/HakanSeven12/OpenCADStudio
- Upstream copyright: Hakan Seven and the Open CAD Studio contributors
- This fork: https://github.com/DF172/DFlow

Modifications made by this fork include rebranding, removal of upstream
packaging and sponsorship workflows, documentation updates, and renaming of
the vendored geometry/file-format crates described below. The full license
text is in [LICENSE](LICENSE); it is unchanged.

## Vendored crates

The following crates under `vendor/` originate from the same upstream author
and are included here as source dependencies. Their license files are kept
verbatim next to each crate, and their original authorship is retained in
each `Cargo.toml`:

| Path | Original name | License | Original source |
| --- | --- | --- | --- |
| `vendor/draftflowcodec` | `opencadcodec` | MPL-2.0 | https://github.com/HakanSeven12/opencadcodec |
| `vendor/draftflowkernel` | `opencadkernel` | MPL-2.0 | https://github.com/HakanSeven12/opencadkernel |
| `vendor/draftflowkernel/crates/draftflowkernel-constraints` | `opencadkernel-constraints` | LGPL-2.1-or-later | part of `opencadkernel` |
| `vendor/draftflowgraph` | `opencadgraph` | MPL-2.0 | https://github.com/HakanSeven12/opencadgraph |

This fork renames the crates and their directories to fit the DraftFlow
naming scheme. No copyright notice has been removed. Modified files remain
subject to the license of the crate they belong to (MPL-2.0 /
LGPL-2.1-or-later respectively).

## Third-party crates

`vendor/iced`, `vendor/iced_aw`, `vendor/winit` and `vendor/cryoglyph` are
upstream third-party projects (iced-rs and rust-windowing), included
unmodified under their own licenses.
