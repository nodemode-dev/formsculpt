//! USD in its text form (.usda), written and read without the USD library.
//!
//! Export: one `Mesh` prim per object under a `Scene` Xform, with points, triangles, vertex normals, the paint colour
//! as `primvars:displayColor`, opacity as `primvars:displayOpacity` when any of it is see-through, and UVs as the
//! face-varying `primvars:st` when the object has them; y up, one unit a metre. Import: every `def Mesh` block's
//! points and faces (polygons split into fans), with displayColor and displayOpacity per vertex or constant. Prim
//! transforms, instancing, subdivision and the binary forms (.usdc, .usdz) are not read.

use std::fmt::Write as _;

use fsc_core::Mesh;

use crate::formats::Named;
use crate::gltf::GltfObject;

/// A valid, unique prim name from an object name.
fn prim_name(name: &str, taken: &mut Vec<String>) -> String {
    let mut base: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if base.is_empty() || base.starts_with(|c: char| c.is_ascii_digit()) {
        base.insert(0, '_');
    }
    let mut out = base.clone();
    let mut k = 1;
    while taken.contains(&out) {
        out = format!("{base}_{k}");
        k += 1;
    }
    taken.push(out.clone());
    out
}

fn list<T>(out: &mut String, items: impl IntoIterator<Item = T>, mut each: impl FnMut(&mut String, T)) {
    out.push('[');
    for (i, item) in items.into_iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        each(out, item);
    }
    out.push(']');
}

pub fn write_usda(objects: &[GltfObject]) -> Vec<u8> {
    let mut out = String::new();
    let _ = writeln!(out, "#usda 1.0\n(\n    defaultPrim = \"Scene\"\n    doc = \"formsculptor {}\"\n    metersPerUnit = 1\n    upAxis = \"Y\"\n)\n", fsc_core::engine_version());
    out.push_str("def Xform \"Scene\"\n{\n");
    let mut taken = Vec::new();
    for o in objects {
        let m = o.mesh;
        let _ = writeln!(out, "    def Mesh \"{}\"\n    {{", prim_name(o.name, &mut taken));
        out.push_str("        int[] faceVertexCounts = ");
        list(&mut out, 0..m.triangle_count(), |s, _| s.push('3'));
        out.push_str("\n        int[] faceVertexIndices = ");
        list(&mut out, &m.indices, |s, i| {
            let _ = write!(s, "{i}");
        });
        out.push_str("\n        normal3f[] normals = ");
        list(&mut out, &m.normals, |s, n| {
            let _ = write!(s, "({}, {}, {})", n[0], n[1], n[2]);
        });
        out.push_str(" (\n            interpolation = \"vertex\"\n        )\n        point3f[] points = ");
        list(&mut out, &m.positions, |s, p| {
            let _ = write!(s, "({}, {}, {})", p[0], p[1], p[2]);
        });
        out.push('\n');
        if m.has_paint() {
            out.push_str("        color3f[] primvars:displayColor = ");
            list(&mut out, &m.paint, |s, p| {
                let _ = write!(s, "({}, {}, {})", p[0], p[1], p[2]);
            });
            out.push_str(" (\n            interpolation = \"vertex\"\n        )\n");
            if m.paint.iter().any(|p| p[5] < 0.999) {
                out.push_str("        float[] primvars:displayOpacity = ");
                list(&mut out, &m.paint, |s, p| {
                    let _ = write!(s, "{}", p[5]);
                });
                out.push_str(" (\n            interpolation = \"vertex\"\n        )\n");
            }
        }
        if m.uvs.len() == m.indices.len() && !m.uvs.is_empty() {
            out.push_str("        texCoord2f[] primvars:st = ");
            list(&mut out, &m.uvs, |s, t| {
                let _ = write!(s, "({}, {})", t[0], t[1]);
            });
            out.push_str(" (\n            interpolation = \"faceVarying\"\n        )\n");
        }
        out.push_str("        uniform token subdivisionScheme = \"none\"\n    }\n");
    }
    out.push_str("}\n");
    out.into_bytes()
}

/// The end of a bracketed or braced span starting at `open` (skipping quoted strings), exclusive.
fn matching(text: &[u8], open: usize) -> Option<usize> {
    let (o, c) = (text[open], if text[open] == b'{' { b'}' } else { b']' });
    let mut depth = 0usize;
    let mut i = open;
    while i < text.len() {
        match text[i] {
            b'"' => {
                i += 1;
                while i < text.len() && text[i] != b'"' {
                    i += 1;
                }
            }
            x if x == o => depth += 1,
            x if x == c => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The numbers of the array attribute `name` in a prim body (tuples flattened), or None when it is not there.
fn numbers(body: &str, name: &str) -> Option<Vec<f64>> {
    let key = format!(" {name} = [");
    let at = body.find(&key)? + key.len() - 1;
    let end = matching(body.as_bytes(), at)?;
    let inner = &body[at + 1..end - 1];
    Some(inner.split(|c: char| c == ',' || c == '(' || c == ')' || c.is_whitespace()).filter(|t| !t.is_empty()).filter_map(|t| t.parse::<f64>().ok()).collect())
}

pub fn read_usda(bytes: &[u8]) -> Result<Vec<Named>, String> {
    if bytes.starts_with(b"PXR-USDC") {
        return Err("this is binary USD (usdc); formsculptor reads the text form, .usda (usdcat converts one to the other)".into());
    }
    if bytes.starts_with(b"PK") {
        return Err("this is a USDZ package; formsculptor reads .usda".into());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "the USD file is not text; formsculptor reads .usda".to_string())?;
    if !text.trim_start().starts_with("#usda") {
        return Err("the file does not start with #usda".into());
    }
    let raw = text.as_bytes();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(found) = text[from..].find("def Mesh \"") {
        let start = from + found + "def Mesh \"".len();
        let name_end = start + text[start..].find('"').ok_or("a Mesh prim's name is not closed")?;
        let name = text[start..name_end].to_string();
        let open = name_end + text[name_end..].find('{').ok_or("a Mesh prim has no body")?;
        let close = matching(raw, open).ok_or("a Mesh prim's body is not closed")?;
        let body = &text[open..close];
        from = close;
        let points = numbers(body, "points").ok_or_else(|| format!("{name} has no points"))?;
        let counts = numbers(body, "faceVertexCounts").ok_or_else(|| format!("{name} has no faceVertexCounts"))?;
        let corners = numbers(body, "faceVertexIndices").ok_or_else(|| format!("{name} has no faceVertexIndices"))?;
        let positions: Vec<[f32; 3]> = points.chunks_exact(3).map(|c| [c[0] as f32, c[1] as f32, c[2] as f32]).collect();
        let mut indices = Vec::with_capacity(corners.len() * 3 / 2);
        let mut at = 0usize;
        for &n in &counts {
            let n = n as usize;
            if at + n > corners.len() {
                return Err(format!("{name}'s faces name more corners than it lists"));
            }
            for k in 1..n.saturating_sub(1) {
                indices.extend_from_slice(&[corners[at] as u32, corners[at + k] as u32, corners[at + k + 1] as u32]);
            }
            at += n;
        }
        if indices.iter().any(|&i| i as usize >= positions.len()) {
            return Err(format!("{name} has a face index past its points"));
        }
        let mut mesh = Mesh::new(positions, indices);
        let vc = mesh.vertex_count();
        let colors = numbers(body, "primvars:displayColor").unwrap_or_default();
        let opacity = numbers(body, "primvars:displayOpacity").unwrap_or_default();
        let per = |list: &[f64], width: usize, v: usize| -> Option<Vec<f64>> {
            if list.len() == vc * width {
                Some(list[v * width..v * width + width].to_vec())
            } else if list.len() == width {
                Some(list.to_vec())
            } else {
                None
            }
        };
        if per(&colors, 3, 0).is_some() || per(&opacity, 1, 0).is_some() {
            mesh.paint = (0..vc)
                .map(|v| {
                    let mut p = fsc_core::DEFAULT_PAINT;
                    if let Some(c) = per(&colors, 3, v) {
                        p[..3].copy_from_slice(&[c[0] as f32, c[1] as f32, c[2] as f32]);
                    }
                    if let Some(a) = per(&opacity, 1, v) {
                        p[5] = (a[0] as f32).clamp(0.0, 1.0);
                    }
                    p
                })
                .collect();
        }
        out.push((name, mesh));
    }
    if out.is_empty() {
        return Err("the USD file has no Mesh prims".into());
    }
    Ok(out)
}
