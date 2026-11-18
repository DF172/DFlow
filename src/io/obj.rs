// Wavefront OBJ mesh importer.
//
// Parses vertex positions, optional normals, and triangle/quad faces.
// Quads are split into two triangles.
// Only the first object/group is imported (no multi-object support needed).

use crate::scene::model::mesh_model::MeshModel;

/// Parse OBJ text into a MeshModel.
/// Returns `None` if the file has no usable geometry.
pub fn parse_obj(src: &str, color: [f32; 4]) -> Option<MeshModel> {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals_raw: Vec<[f32; 3]> = Vec::new();
    // Each face vertex: (position, normal_idx_opt). The position is resolved
    // while the face is parsed, so a corner the file cannot back up drops the
    // face instead of arriving as the world origin.
    let mut face_verts: Vec<([f32; 3], Option<usize>)> = Vec::new();

    for line in src.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        let mut parts = line.split_whitespace();
        let keyword = parts.next().unwrap_or("");

        match keyword {
            "v" => {
                let vals: Vec<f32> = parts.filter_map(|s| s.parse().ok()).collect();
                if vals.len() >= 3 {
                    // OBJ is Y-up right-handed; DraftFlowCAD's world is Z-up.
                    // Rotate +90° about X so OBJ Y→world Z (up): (x, y, z) → (x, -z, y).
                    positions.push([vals[0], -vals[2], vals[1]]);
                }
            }
            "vn" => {
                let vals: Vec<f32> = parts.filter_map(|s| s.parse().ok()).collect();
                if vals.len() >= 3 {
                    normals_raw.push([vals[0], -vals[2], vals[1]]);
                }
            }
            "f" => {
                // Collect vertex descriptors "v", "v/vt", "v/vt/vn", "v//vn"
                let descs: Vec<(usize, Option<usize>)> = parts
                    .filter_map(|token| {
                        let mut it = token.split('/');
                        let pos_i: usize = it.next()?.parse::<i32>().ok()
                            .map(|i| if i < 0 { positions.len() as i32 + i } else { i - 1 })? as usize;
                        it.next(); // skip vt
                        let norm_i = it.next().and_then(|s| s.parse::<i32>().ok())
                            .map(|i| if i < 0 { normals_raw.len() as i32 + i } else { i - 1 } as usize);
                        Some((pos_i, norm_i))
                    })
                    .collect();
                // Resolve every corner before triangulating. A face citing a
                // position the file never defined cannot be placed; pinning
                // its missing corner to (0,0,0) drew a spike across the model
                // while the import still reported success.
                let resolved: Vec<([f32; 3], Option<usize>)> = descs
                    .iter()
                    .filter_map(|(pos_i, norm_i)| {
                        positions.get(*pos_i).copied().map(|pos| (pos, *norm_i))
                    })
                    .collect();
                if resolved.len() != descs.len() {
                    continue;
                }
                // A fan is only right for convex faces; floor-plan n-gons
                // are concave, so larger faces go through the kernel's
                // triangulator and fall back to the fan if it declines. (#680)
                let corners: Vec<[f64; 3]> = resolved
                    .iter()
                    .map(|(p, _)| [p[0] as f64, p[1] as f64, p[2] as f64])
                    .collect();
                let triangles = if resolved.len() > 3 {
                    crate::entities::mesh::triangulate_planar_indices(&corners)
                } else {
                    Vec::new()
                };
                if triangles.is_empty() {
                    for k in 1..(resolved.len() as isize - 1) {
                        face_verts.push(resolved[0]);
                        face_verts.push(resolved[k as usize]);
                        face_verts.push(resolved[k as usize + 1]);
                    }
                } else {
                    for triangle in triangles {
                        face_verts.extend(triangle.map(|index| resolved[index]));
                    }
                }
            }
            _ => {}
        }
    }

    if positions.is_empty() || face_verts.is_empty() {
        return None;
    }

    // Build flat (un-indexed) vertex + normal arrays.
    let mut verts: Vec<[f32; 3]> = Vec::with_capacity(face_verts.len());
    let mut norms: Vec<[f32; 3]> = Vec::with_capacity(face_verts.len());
    let mut indices: Vec<u32> = Vec::with_capacity(face_verts.len());
    let mut has_normal: Vec<bool> = Vec::with_capacity(face_verts.len());

    for (vi, (pos, norm_i)) in face_verts.iter().enumerate() {
        verts.push(*pos);
        let norm = norm_i
            .and_then(|ni| normals_raw.get(ni).copied())
            .filter(|n| {
                n.iter().all(|value| value.is_finite())
                    && n.iter().map(|value| value * value).sum::<f32>() > 1e-12
            });
        has_normal.push(norm.is_some());
        norms.push(norm.unwrap_or([0.0, 0.0, 0.0]));
        indices.push(vi as u32);
    }

    // A vertex the file gives no usable normal (the file lists none, the face
    // omits its normal indices, or an index is out of range) takes its face
    // normal instead of staying zero.
    for tri in indices.chunks_exact(3) {
        if tri.iter().all(|&v| has_normal[v as usize]) {
            continue;
        }
        let a = verts[tri[0] as usize];
        let b = verts[tri[1] as usize];
        let c = verts[tri[2] as usize];
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let nx = ab[1] * ac[2] - ab[2] * ac[1];
        let ny = ab[2] * ac[0] - ab[0] * ac[2];
        let nz = ab[0] * ac[1] - ab[1] * ac[0];
        let len = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-12);
        let n = [nx / len, ny / len, nz / len];
        for &v in tri {
            if !has_normal[v as usize] {
                norms[v as usize] = n;
            }
        }
    }

    Some(MeshModel {
        name: String::new(),
        verts,
        verts_low: Vec::new(),
        normals: norms,
        indices,
        triangle_material_handles: Vec::new(),
        triangle_colors: Vec::new(),
        color,
        selected: false,
    })
}

#[cfg(test)]
mod tests {
    use super::parse_obj;

    const COLOR: [f32; 4] = [0.7, 0.7, 0.85, 1.0];

    /// A unit square in the OBJ ground plane (Y-up), so it lands in world XY
    /// with its counter-clockwise faces pointing up +Z.
    const SQUARE: &str = "v 0 0 0\nv 1 0 0\nv 1 0 -1\nv 0 0 -1\n";

    fn assert_normals(normals: &[[f32; 3]], expected: [f32; 3]) {
        for (i, n) in normals.iter().enumerate() {
            let close = n.iter().zip(expected).all(|(a, b)| (a - b).abs() < 1e-6);
            assert!(close, "vertex {i}: expected normal {expected:?}, got {n:?}");
        }
    }

    /// OBJ lets a face omit normal indices even when the file lists normals.
    /// That face used to get a zero normal instead of its face normal.
    #[test]
    fn a_face_without_normal_indices_gets_its_face_normal() {
        let src = format!("{SQUARE}vn 0 1 0\nf 1//1 2//1 3//1\nf 1 3 4\n");
        let mesh = parse_obj(&src, COLOR).expect("mesh");
        assert_eq!(mesh.normals.len(), 6);
        assert_normals(&mesh.normals[3..], [0.0, 0.0, 1.0]);
    }

    /// A normal index past the end of the list is treated like a missing one.
    #[test]
    fn an_out_of_range_normal_index_gets_the_face_normal() {
        let src = format!("{SQUARE}vn 0 1 0\nf 1//7 2//7 3//7\n");
        let mesh = parse_obj(&src, COLOR).expect("mesh");
        assert_normals(&mesh.normals, [0.0, 0.0, 1.0]);
    }

    /// A position index the file never defined used to become the world origin,
    /// so the mesh grew a spike across the model and the import still reported
    /// success. Normals have a remedy (the face normal above); coordinates
    /// don't, so such a face is dropped whole.
    #[test]
    fn a_face_citing_an_undefined_position_is_dropped_not_pinned_to_the_origin() {
        // Deliberately avoids `SQUARE`, whose first vertex is (0,0,0): here an
        // origin vertex can only come from the bug.
        let plane = "v 1 0 0\nv 2 0 0\nv 2 0 -1\n";
        let forward = parse_obj(&format!("{plane}f 1 2 3\nf 1 2 9\n"), COLOR)
            .expect("the placeable face survives");
        assert_eq!(forward.verts.len(), 3, "only the real face imports");
        assert!(
            forward.verts.iter().all(|v| *v != [0.0, 0.0, 0.0]),
            "no corner may sit at the origin: {:?}",
            forward.verts
        );

        let negative = parse_obj(&format!("{plane}f 1 2 3\nf -1 -2 -9\n"), COLOR)
            .expect("the placeable face survives");
        assert_eq!(
            negative.verts.len(),
            3,
            "-9 counts back past the start of the list and drops its face too"
        );

        assert!(
            parse_obj("v 1 0 0\nv 2 0 0\nf 1 2 9\n", COLOR).is_none(),
            "a file whose only face cannot be placed has no usable geometry"
        );
    }

    #[test]
    fn a_zero_normal_gets_the_face_normal() {
        let src = format!("{SQUARE}vn 0 0 0\nf 1//1 2//1 3//1\n");
        let mesh = parse_obj(&src, COLOR).expect("mesh");
        assert_normals(&mesh.normals, [0.0, 0.0, 1.0]);
    }

    /// Only the vertex with a bad index is filled; its neighbours keep theirs.
    #[test]
    fn only_the_vertex_without_a_normal_is_filled() {
        let src = format!("{SQUARE}vn 1 0 0\nf 1//1 2//9 3//1\n");
        let mesh = parse_obj(&src, COLOR).expect("mesh");
        assert_normals(&mesh.normals[0..1], [1.0, 0.0, 0.0]);
        assert_normals(&mesh.normals[1..2], [0.0, 0.0, 1.0]);
        assert_normals(&mesh.normals[2..3], [1.0, 0.0, 0.0]);
    }

    /// Normals the file supplies are kept as written (converted to Z-up).
    #[test]
    fn supplied_normals_are_kept() {
        let src = format!("{SQUARE}vn 1 0 0\nf 1//1 2//1 3//1\nf 1//1 3//1 4//1\n");
        let mesh = parse_obj(&src, COLOR).expect("mesh");
        assert_normals(&mesh.normals, [1.0, 0.0, 0.0]);
    }

    /// A file without any normals still gets face normals.
    #[test]
    fn a_file_without_normals_gets_face_normals() {
        let src = format!("{SQUARE}f 1 2 3 4\n");
        let mesh = parse_obj(&src, COLOR).expect("mesh");
        assert_eq!(mesh.normals.len(), 6);
        assert_normals(&mesh.normals, [0.0, 0.0, 1.0]);
    }
}
