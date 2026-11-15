use std::io::{Cursor, Read, Write};

use fsc_core::Mesh;

use crate::bin::{compacted, welded};

pub type Named = (String, Mesh);

fn num(tok: Option<&str>, what: &str) -> Result<f32, String> {
    tok.and_then(|t| t.parse::<f32>().ok()).ok_or_else(|| format!("a {what} line has a bad number"))
}

/// The diffuse colours (Kd r g b) of the materials in an MTL file, by name.
fn mtl_colors(text: &str) -> std::collections::HashMap<String, [f32; 3]> {
    let mut out = std::collections::HashMap::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("newmtl") => current = Some(it.collect::<Vec<_>>().join(" ")),
            Some("Kd") => {
                let rgb: Vec<f32> = it.filter_map(|t| t.parse::<f32>().ok()).collect();
                if let (Some(name), true) = (&current, rgb.len() >= 3) {
                    out.insert(name.clone(), [rgb[0].clamp(0.0, 1.0), rgb[1].clamp(0.0, 1.0), rgb[2].clamp(0.0, 1.0)]);
                }
            }
            _ => {}
        }
    }
    out
}

/// Reads an OBJ, and the MTL files its mtllib lines name through `resolve` (files beside it): when the vertices carry no
/// colour of their own, each vertex takes the diffuse colour (Kd) of the material its faces use (usemtl), so a model
/// coloured by materials keeps its colours as paint. A vertex shared by faces of two materials takes the later one's.
pub fn read_obj_with(bytes: &[u8], resolve: crate::gltf::Resolve) -> Result<Vec<Named>, String> {
    let text = String::from_utf8_lossy(bytes);
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 3]> = Vec::new();
    let mut colored = true;
    let mut parts: Vec<(String, Vec<u32>)> = vec![("Imported".into(), Vec::new())];
    let mut materials: std::collections::HashMap<String, [f32; 3]> = std::collections::HashMap::new();
    let mut using: Option<[f32; 3]> = None;
    let mut by_material: Vec<(u32, [f32; 3])> = Vec::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        match it.next() {
            Some("mtllib") => {
                for name in line.trim_start()[6..].split_whitespace() {
                    if let Ok(found) = resolve(name) {
                        materials.extend(mtl_colors(&String::from_utf8_lossy(&found)));
                    }
                }
            }
            Some("usemtl") => using = materials.get(&it.collect::<Vec<_>>().join(" ")).copied(),
            Some("v") => {
                positions.push([num(it.next(), "v")?, num(it.next(), "v")?, num(it.next(), "v")?]);
                let rgb: Vec<f32> = it.filter_map(|t| t.parse::<f32>().ok()).collect();
                if rgb.len() >= 3 && colored {
                    colors.push([rgb[0].clamp(0.0, 1.0), rgb[1].clamp(0.0, 1.0), rgb[2].clamp(0.0, 1.0)]);
                } else {
                    colored = false;
                }
            }
            Some("o") | Some("g") => {
                let name = it.collect::<Vec<_>>().join(" ");
                if parts.last().is_some_and(|p| p.1.is_empty()) {
                    parts.pop();
                }
                parts.push((if name.is_empty() { "Imported".into() } else { name }, Vec::new()));
            }
            Some("f") => {
                let mut poly = Vec::new();
                for tok in it {
                    let first = tok.split('/').next().unwrap_or("");
                    let i: i64 = first.parse().map_err(|_| "a face has a bad index".to_string())?;
                    let idx = if i < 0 { positions.len() as i64 + i } else { i - 1 };
                    if idx < 0 || idx as usize >= positions.len() {
                        return Err("a face points past the vertex list".into());
                    }
                    poly.push(idx as u32);
                }
                if let Some(c) = using {
                    by_material.extend(poly.iter().map(|&v| (v, c)));
                }
                let face = &mut parts.last_mut().expect("one part").1;
                for k in 1..poly.len().saturating_sub(1) {
                    face.extend_from_slice(&[poly[0], poly[k], poly[k + 1]]);
                }
            }
            _ => {}
        }
    }
    if !colored {
        colors.clear();
    }
    // Material colours, where the vertices had none of their own; vertices of faces with no coloured material keep the
    // default paint colour.
    if colors.is_empty() && !by_material.is_empty() {
        let d = fsc_core::DEFAULT_PAINT;
        colors = vec![[d[0], d[1], d[2]]; positions.len()];
        for (v, c) in by_material {
            colors[v as usize] = c;
        }
    }
    let out: Vec<Named> = parts.into_iter().filter(|p| !p.1.is_empty()).map(|(n, f)| (n, compacted(&positions, &colors, &f))).collect();
    if out.is_empty() {
        return Err("the OBJ file has no faces".into());
    }
    Ok(out)
}

pub fn write_obj(objects: &[(&str, &Mesh)]) -> Vec<u8> {
    let mut s = String::from("# formsculpt\n");
    let (mut base, mut uv_base) = (1, 1);
    for (name, mesh) in objects {
        s.push_str(&format!("o {name}\n"));
        for (v, p) in mesh.positions.iter().enumerate() {
            match mesh.paint.get(v) {
                Some(c) => s.push_str(&format!("v {} {} {} {} {} {}\n", p[0], p[1], p[2], c[0], c[1], c[2])),
                None => s.push_str(&format!("v {} {} {}\n", p[0], p[1], p[2])),
            }
        }
        // UVs are per triangle corner: one vt per corner, faces as v/vt (texturing apps read them).
        if mesh.uvs.len() == mesh.indices.len() && !mesh.uvs.is_empty() {
            for t in &mesh.uvs {
                s.push_str(&format!("vt {} {}\n", t[0], t[1]));
            }
            for (k, t) in mesh.indices.as_chunks::<3>().0.iter().enumerate() {
                let c = uv_base + 3 * k as u32;
                s.push_str(&format!("f {}/{} {}/{} {}/{}\n", t[0] + base, c, t[1] + base, c + 1, t[2] + base, c + 2));
            }
            uv_base += mesh.uvs.len() as u32;
            base += mesh.vertex_count() as u32;
            continue;
        }
        match mesh.quad_faces() {
            Some(quads) => {
                for q in quads {
                    s.push_str(&format!("f {} {} {} {}\n", q[0] + base, q[1] + base, q[2] + base, q[3] + base));
                }
            }
            None => {
                for t in mesh.indices.as_chunks::<3>().0 {
                    s.push_str(&format!("f {} {} {}\n", t[0] + base, t[1] + base, t[2] + base));
                }
            }
        }
        base += mesh.vertex_count() as u32;
    }
    s.into_bytes()
}

pub fn read_stl(bytes: &[u8]) -> Result<Vec<Named>, String> {
    let mut tris: Vec<[f32; 3]> = Vec::new();
    let binary_len = (bytes.len() >= 84).then(|| 84 + 50 * u32::from_le_bytes([bytes[80], bytes[81], bytes[82], bytes[83]]) as usize);
    if binary_len == Some(bytes.len()) {
        let n = (bytes.len() - 84) / 50;
        for t in 0..n {
            let at = 84 + 50 * t + 12;
            for v in 0..3 {
                let o = at + 12 * v;
                let f = |k: usize| f32::from_le_bytes([bytes[o + 4 * k], bytes[o + 4 * k + 1], bytes[o + 4 * k + 2], bytes[o + 4 * k + 3]]);
                tris.push([f(0), f(1), f(2)]);
            }
        }
    } else {
        let text = String::from_utf8_lossy(bytes);
        for line in text.lines() {
            let mut it = line.split_whitespace();
            if it.next() == Some("vertex") {
                tris.push([num(it.next(), "vertex")?, num(it.next(), "vertex")?, num(it.next(), "vertex")?]);
            }
        }
        if !tris.len().is_multiple_of(3) {
            return Err("the ASCII STL has an incomplete facet".into());
        }
    }
    if tris.is_empty() {
        return Err("the STL file has no triangles".into());
    }
    Ok(vec![("Imported".into(), welded(&tris))])
}

pub fn write_stl(mesh: &Mesh) -> Vec<u8> {
    let mut out = vec![0u8; 80];
    out[..18].copy_from_slice(b"formsculpt binary ");
    out.extend_from_slice(&(mesh.triangle_count() as u32).to_le_bytes());
    for t in 0..mesh.triangle_count() {
        let [a, b, c] = mesh.triangle(t);
        let n = (mesh.position(b) - mesh.position(a)).cross(mesh.position(c) - mesh.position(a)).normalize_or_zero();
        for x in n.to_array().iter().chain(&mesh.positions[a as usize]).chain(&mesh.positions[b as usize]).chain(&mesh.positions[c as usize]) {
            out.extend_from_slice(&x.to_le_bytes());
        }
        out.extend_from_slice(&[0, 0]);
    }
    out
}

fn ply_size(ty: &str) -> Option<usize> {
    Some(match ty {
        "char" | "uchar" | "int8" | "uint8" => 1,
        "short" | "ushort" | "int16" | "uint16" => 2,
        "int" | "uint" | "int32" | "uint32" | "float" | "float32" => 4,
        "double" | "float64" => 8,
        _ => return None,
    })
}

fn ply_read(ty: &str, b: &[u8]) -> f64 {
    match ty {
        "char" | "int8" => b[0] as i8 as f64,
        "uchar" | "uint8" => b[0] as f64,
        "short" | "int16" => i16::from_le_bytes([b[0], b[1]]) as f64,
        "ushort" | "uint16" => u16::from_le_bytes([b[0], b[1]]) as f64,
        "int" | "int32" => i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
        "uint" | "uint32" => u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
        "float" | "float32" => f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64,
        _ => f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]),
    }
}

struct PlyElement {
    name: String,
    count: usize,
    props: Vec<(String, String, Option<String>)>,
}

pub fn read_ply(bytes: &[u8]) -> Result<Vec<Named>, String> {
    let end = bytes.windows(10).position(|w| w == b"end_header").ok_or("the PLY file has no header end")?;
    let header = String::from_utf8_lossy(&bytes[..end]);
    let mut body = end + 10;
    if bytes.get(body) == Some(&b'\r') {
        body += 1;
    }
    if bytes.get(body) == Some(&b'\n') {
        body += 1;
    }
    let mut format = "";
    let mut elements: Vec<PlyElement> = Vec::new();
    for line in header.lines() {
        let t: Vec<&str> = line.split_whitespace().collect();
        match t.as_slice() {
            ["format", f, ..] => format = if *f == "ascii" { "ascii" } else if *f == "binary_little_endian" { "le" } else { "other" },
            ["element", name, count] => elements.push(PlyElement { name: name.to_string(), count: count.parse().map_err(|_| "bad element count")?, props: Vec::new() }),
            ["property", "list", ct, it, name] => elements.last_mut().ok_or("property before element")?.props.push((name.to_string(), ct.to_string(), Some(it.to_string()))),
            ["property", ty, name] => elements.last_mut().ok_or("property before element")?.props.push((name.to_string(), ty.to_string(), None)),
            _ => {}
        }
    }
    if format == "other" || format.is_empty() {
        return Err("only ASCII and binary little-endian PLY are read".into());
    }
    if elements.iter().any(|e| e.count > if e.props.is_empty() { 0 } else { bytes.len() - body }) {
        return Err("the PLY header names more elements than the file holds".into());
    }
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 3]> = Vec::new();
    let has_rgb = elements.iter().any(|e| e.name == "vertex" && e.props.iter().any(|p| p.0 == "red"));
    let mut faces: Vec<u32> = Vec::new();
    let text = if format == "ascii" { String::from_utf8_lossy(&bytes[body..]).into_owned() } else { String::new() };
    let mut words = text.split_whitespace();
    let mut at = body;
    for el in &elements {
        for _ in 0..el.count {
            let mut xyz = [0f32; 3];
            let mut rgb = [0.8f32; 3];
            for (name, ty, list) in &el.props {
                let mut take = |ty: &str| -> Result<f64, String> {
                    if format == "ascii" {
                        words.next().and_then(|w| w.parse::<f64>().ok()).ok_or_else(|| "the PLY body ends early".to_string())
                    } else {
                        let n = ply_size(ty).ok_or_else(|| format!("unknown PLY type {ty}"))?;
                        let slice = bytes.get(at..at + n).ok_or("the PLY body ends early")?;
                        at += n;
                        Ok(ply_read(ty, slice))
                    }
                };
                match list {
                    Some(it) => {
                        let n = take(ty)? as usize;
                        if n > bytes.len() {
                            return Err("the PLY body ends early".into());
                        }
                        let mut poly = Vec::with_capacity(n);
                        for _ in 0..n {
                            poly.push(take(it)? as u32);
                        }
                        if el.name == "face" {
                            for k in 1..n.saturating_sub(1) {
                                faces.extend_from_slice(&[poly[0], poly[k], poly[k + 1]]);
                            }
                        }
                    }
                    None => {
                        let v = take(ty)? as f32;
                        match name.as_str() {
                            "x" => xyz[0] = v,
                            "y" => xyz[1] = v,
                            "z" => xyz[2] = v,
                            "red" | "green" | "blue" => {
                                let scale = match ty.as_str() {
                                    "uchar" | "uint8" | "char" | "int8" => 255.0,
                                    "ushort" | "uint16" | "short" | "int16" => 65535.0,
                                    _ => 1.0,
                                };
                                let k = match name.as_str() {
                                    "red" => 0,
                                    "green" => 1,
                                    _ => 2,
                                };
                                rgb[k] = (v / scale).clamp(0.0, 1.0);
                            }
                            _ => {}
                        }
                    }
                }
            }
            if el.name == "vertex" {
                positions.push(xyz);
                if has_rgb {
                    colors.push(rgb);
                }
            }
        }
    }
    if faces.iter().any(|&i| i as usize >= positions.len()) {
        return Err("a PLY face points past the vertex list".into());
    }
    if faces.is_empty() {
        return Err("the PLY file has no faces".into());
    }
    let mut mesh = Mesh::new(positions, faces);
    if has_rgb {
        mesh.paint = colors.into_iter().map(crate::bin::with_color).collect();
    }
    Ok(vec![("Imported".into(), mesh)])
}

pub fn write_ply(mesh: &Mesh) -> Vec<u8> {
    let rgb = if mesh.has_paint() { "property uchar red\nproperty uchar green\nproperty uchar blue\n" } else { "" };
    let mut out = format!(
        "ply\nformat binary_little_endian 1.0\ncomment formsculpt\nelement vertex {}\nproperty float x\nproperty float y\nproperty float z\n{rgb}element face {}\nproperty list uchar int vertex_indices\nend_header\n",
        mesh.vertex_count(),
        mesh.triangle_count()
    )
    .into_bytes();
    for (v, p) in mesh.positions.iter().enumerate() {
        for c in p {
            out.extend_from_slice(&c.to_le_bytes());
        }
        if let Some(c) = mesh.paint.get(v) {
            out.extend(c[..3].iter().map(|x| (x.clamp(0.0, 1.0) * 255.0).round() as u8));
        }
    }
    for t in mesh.indices.as_chunks::<3>().0 {
        out.push(3);
        for i in t {
            out.extend_from_slice(&(*i as i32).to_le_bytes());
        }
    }
    out
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!(" {name}=\"");
    let at = tag.find(&key)? + key.len();
    let end = tag[at..].find('"')? + at;
    Some(tag[at..end].to_string())
}

pub fn read_3mf(bytes: &[u8]) -> Result<Vec<Named>, String> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("the 3MF is not a readable zip: {e}"))?;
    let name = (0..zip.len()).filter_map(|i| zip.by_index(i).ok().map(|f| f.name().to_string())).find(|n| n.to_lowercase().ends_with(".model")).ok_or("the 3MF has no model part")?;
    let mut xml = String::new();
    zip.by_name(&name).map_err(|e| e.to_string())?.take(crate::project::ENTRY_MAX + 1).read_to_string(&mut xml).map_err(|e| e.to_string())?;
    if xml.len() as u64 > crate::project::ENTRY_MAX {
        return Err("the 3MF model part is too large to read".into());
    }
    let mut out = Vec::new();
    for chunk in xml.split("<object").skip(1) {
        let head = &chunk[..chunk.find('>').unwrap_or(0)];
        let obj_name = attr(head, "name").unwrap_or_else(|| format!("Object {}", attr(head, "id").unwrap_or_default()));
        let mut positions = Vec::new();
        let mut faces = Vec::new();
        for v in chunk.split("<vertex").skip(1) {
            if !v.starts_with(char::is_whitespace) {
                continue;
            }
            let tag = &v[..v.find('>').unwrap_or(v.len())];
            let g = |k: &str| attr(tag, k).and_then(|s| s.parse::<f32>().ok()).ok_or_else(|| "a 3MF vertex has a bad coordinate".to_string());
            positions.push([g("x")?, g("y")?, g("z")?]);
        }
        for t in chunk.split("<triangle").skip(1) {
            if !t.starts_with(char::is_whitespace) {
                continue;
            }
            let tag = &t[..t.find('>').unwrap_or(t.len())];
            let g = |k: &str| attr(tag, k).and_then(|s| s.parse::<u32>().ok()).ok_or_else(|| "a 3MF triangle has a bad index".to_string());
            faces.extend_from_slice(&[g("v1")?, g("v2")?, g("v3")?]);
        }
        if faces.iter().any(|&i| i as usize >= positions.len()) {
            return Err("a 3MF triangle points past its vertices".into());
        }
        if !faces.is_empty() {
            out.push((obj_name, Mesh::new(positions, faces)));
        }
        if chunk.contains("</resources>") {
            break;
        }
    }
    if out.is_empty() {
        return Err("the 3MF has no mesh objects".into());
    }
    Ok(out)
}

pub fn write_3mf(objects: &[(&str, &Mesh)]) -> Result<Vec<u8>, String> {
    let mut model = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<model unit=\"millimeter\" xml:lang=\"en-US\" xmlns=\"http://schemas.microsoft.com/3dmanufacturing/core/2015/02\">\n<resources>\n");
    for (k, (name, mesh)) in objects.iter().enumerate() {
        let safe = name.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;");
        model.push_str(&format!("<object id=\"{}\" type=\"model\" name=\"{safe}\"><mesh><vertices>\n", k + 1));
        for p in &mesh.positions {
            model.push_str(&format!("<vertex x=\"{}\" y=\"{}\" z=\"{}\"/>\n", p[0], p[1], p[2]));
        }
        model.push_str("</vertices><triangles>\n");
        for t in mesh.indices.as_chunks::<3>().0 {
            model.push_str(&format!("<triangle v1=\"{}\" v2=\"{}\" v3=\"{}\"/>\n", t[0], t[1], t[2]));
        }
        model.push_str("</triangles></mesh></object>\n");
    }
    model.push_str("</resources>\n<build>\n");
    for k in 0..objects.len() {
        model.push_str(&format!("<item objectid=\"{}\"/>\n", k + 1));
    }
    model.push_str("</build>\n</model>\n");
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let parts = [
        ("[Content_Types].xml", "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"model\" ContentType=\"application/vnd.ms-package.3dmanufacturing-3dmodel+xml\"/></Types>".to_string()),
        ("_rels/.rels", "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Target=\"/3D/3dmodel.model\" Id=\"rel0\" Type=\"http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel\"/></Relationships>".to_string()),
        ("3D/3dmodel.model", model),
    ];
    for (name, body) in parts {
        zip.start_file(name, opts).map_err(|e| e.to_string())?;
        zip.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
    }
    Ok(zip.finish().map_err(|e| e.to_string())?.into_inner())
}
