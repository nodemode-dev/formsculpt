use base64::Engine;
use fsc_core::Mesh;
use glam::{Mat4, Quat, Vec3};
use serde_json::{json, Value};

use crate::formats::Named;

pub struct Target<'a> {
    pub name: &'a str,
    pub weight: f32,
    pub delta: &'a [[f32; 3]],
}

pub struct GltfObject<'a> {
    pub name: &'a str,
    pub mesh: &'a Mesh,
    pub targets: Vec<Target<'a>>,
}

fn pad(buf: &mut Vec<u8>, with: u8) {
    while buf.len() % 4 != 0 {
        buf.push(with);
    }
}

pub fn write_glb(objects: &[GltfObject]) -> Vec<u8> {
    let mut bin: Vec<u8> = Vec::new();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut meshes = Vec::new();
    let mut nodes = Vec::new();
    let mut add_vec3 = |bin: &mut Vec<u8>, data: &[[f32; 3]], minmax: bool| -> usize {
        pad(bin, 0);
        let offset = bin.len();
        let (mut lo, mut hi) = ([f32::INFINITY; 3], [f32::NEG_INFINITY; 3]);
        for p in data {
            for k in 0..3 {
                bin.extend_from_slice(&p[k].to_le_bytes());
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        views.push(json!({"buffer": 0, "byteOffset": offset, "byteLength": data.len() * 12, "target": 34962}));
        let mut acc = json!({"bufferView": views.len() - 1, "componentType": 5126, "count": data.len(), "type": "VEC3"});
        if minmax {
            acc["min"] = json!(lo);
            acc["max"] = json!(hi);
        }
        accessors.push(acc);
        accessors.len() - 1
    };
    let mut index_accessors = Vec::new();
    for o in objects {
        let base: Vec<[f32; 3]> = o
            .mesh
            .positions
            .iter()
            .enumerate()
            .map(|(v, p)| {
                let mut q = Vec3::from_array(*p);
                for t in &o.targets {
                    q -= Vec3::from_array(t.delta[v]) * t.weight;
                }
                q.to_array()
            })
            .collect();
        let pos = add_vec3(&mut bin, &base, true);
        let nor = add_vec3(&mut bin, &o.mesh.normals, false);
        let targets: Vec<usize> = o.targets.iter().map(|t| add_vec3(&mut bin, t.delta, true)).collect();
        let color = o.mesh.has_paint().then(|| {
            let rgb: Vec<[f32; 3]> = o.mesh.paint.iter().map(|p| [p[0], p[1], p[2]]).collect();
            add_vec3(&mut bin, &rgb, false)
        });
        index_accessors.push((pos, nor, targets, color));
    }
    let mut materials = Vec::new();
    for (k, o) in objects.iter().enumerate() {
        pad(&mut bin, 0);
        let offset = bin.len();
        for i in &o.mesh.indices {
            bin.extend_from_slice(&i.to_le_bytes());
        }
        views.push(json!({"buffer": 0, "byteOffset": offset, "byteLength": o.mesh.indices.len() * 4, "target": 34963}));
        accessors.push(json!({"bufferView": views.len() - 1, "componentType": 5125, "count": o.mesh.indices.len(), "type": "SCALAR"}));
        let (pos, nor, targets, color) = &index_accessors[k];
        let mut prim = json!({"attributes": {"POSITION": pos, "NORMAL": nor}, "indices": accessors.len() - 1, "mode": 4, "material": materials.len()});
        let n = o.mesh.vertex_count().max(1) as f32;
        let (rough, metal) = if o.mesh.has_paint() {
            (o.mesh.paint.iter().map(|p| p[3]).sum::<f32>() / n, o.mesh.paint.iter().map(|p| p[4]).sum::<f32>() / n)
        } else {
            (fsc_core::DEFAULT_PAINT[3], fsc_core::DEFAULT_PAINT[4])
        };
        let tint = if color.is_some() { [1.0, 1.0, 1.0, 1.0] } else { [0.8, 0.8, 0.8, 1.0] };
        materials.push(json!({"name": o.name, "pbrMetallicRoughness": {"baseColorFactor": tint, "metallicFactor": metal, "roughnessFactor": rough}}));
        if let Some(c) = color {
            prim["attributes"]["COLOR_0"] = json!(c);
        }
        let mut mesh = json!({"name": o.name, "primitives": []});
        if !targets.is_empty() {
            prim["targets"] = json!(targets.iter().map(|t| json!({"POSITION": t})).collect::<Vec<_>>());
            mesh["weights"] = json!(o.targets.iter().map(|t| t.weight).collect::<Vec<_>>());
            mesh["extras"] = json!({"targetNames": o.targets.iter().map(|t| t.name).collect::<Vec<_>>()});
        }
        mesh["primitives"] = json!([prim]);
        meshes.push(mesh);
        nodes.push(json!({"name": o.name, "mesh": k}));
    }
    pad(&mut bin, 0);
    let doc = json!({
        "asset": {"version": "2.0", "generator": format!("formsculptor {}", fsc_core::engine_version())},
        "scene": 0,
        "scenes": [{"nodes": (0..objects.len()).collect::<Vec<_>>()}],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
        "buffers": [{"byteLength": bin.len()}],
        "bufferViews": views,
        "accessors": accessors,
    });
    let mut text = serde_json::to_vec(&doc).expect("json");
    pad(&mut text, b' ');
    let total = 12 + 8 + text.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&text);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin);
    out
}

fn buffer_bytes(doc: &Value, glb_bin: Option<&[u8]>) -> Result<Vec<Vec<u8>>, String> {
    let mut out = Vec::new();
    for (k, b) in doc["buffers"].as_array().cloned().unwrap_or_default().iter().enumerate() {
        match b["uri"].as_str() {
            None if k == 0 => out.push(glb_bin.ok_or("buffer 0 has no data")?.to_vec()),
            Some(uri) if uri.starts_with("data:") => {
                let data = uri.split(",").nth(1).ok_or("a data URI has no data")?;
                out.push(base64::engine::general_purpose::STANDARD.decode(data).map_err(|e| format!("a data URI is not base64: {e}"))?);
            }
            Some(uri) => return Err(format!("the glTF refers to an outside file ({uri}); export it as GLB or embedded glTF")),
            None => return Err("a buffer has no data".into()),
        }
    }
    Ok(out)
}

fn accessor(doc: &Value, buffers: &[Vec<u8>], i: usize) -> Result<Vec<f64>, String> {
    let a = &doc["accessors"][i];
    let count = a["count"].as_u64().ok_or("an accessor has no count")? as usize;
    let comps = match a["type"].as_str() {
        Some("SCALAR") => 1,
        Some("VEC3") => 3,
        Some("VEC2") => 2,
        Some("VEC4") => 4,
        _ => return Err("an accessor type is not supported".into()),
    };
    let (size, read): (usize, fn(&[u8]) -> f64) = match a["componentType"].as_u64() {
        Some(5126) => (4, |b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64),
        Some(5125) => (4, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64),
        Some(5123) => (2, |b| u16::from_le_bytes([b[0], b[1]]) as f64),
        Some(5121) => (1, |b| b[0] as f64),
        _ => return Err("an accessor component type is not supported".into()),
    };
    let Some(view_index) = a["bufferView"].as_u64() else { return Ok(vec![0.0; count * comps]) };
    let view = &doc["bufferViews"][view_index as usize];
    let buf = buffers.get(view["buffer"].as_u64().unwrap_or(0) as usize).ok_or("a view points at a missing buffer")?;
    let base = view["byteOffset"].as_u64().unwrap_or(0) as usize + a["byteOffset"].as_u64().unwrap_or(0) as usize;
    let stride = view["byteStride"].as_u64().map(|s| s as usize).unwrap_or(size * comps);
    let mut out = Vec::with_capacity(count * comps);
    for e in 0..count {
        for c in 0..comps {
            let at = base + e * stride + c * size;
            let slice = buf.get(at..at + size).ok_or("an accessor reads past its buffer")?;
            out.push(read(slice));
        }
    }
    Ok(out)
}

fn local(node: &Value) -> Mat4 {
    if let Some(m) = node["matrix"].as_array() {
        let v: Vec<f32> = m.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect();
        if v.len() == 16 {
            return Mat4::from_cols_slice(&v);
        }
    }
    let arr = |k: &str, d: Vec<f32>| -> Vec<f32> { node[k].as_array().map(|a| a.iter().map(|x| x.as_f64().unwrap_or(0.0) as f32).collect()).unwrap_or(d) };
    let t = arr("translation", vec![0.0, 0.0, 0.0]);
    let r = arr("rotation", vec![0.0, 0.0, 0.0, 1.0]);
    let s = arr("scale", vec![1.0, 1.0, 1.0]);
    Mat4::from_scale_rotation_translation(Vec3::new(s[0], s[1], s[2]), Quat::from_xyzw(r[0], r[1], r[2], r[3]), Vec3::new(t[0], t[1], t[2]))
}

pub fn read_gltf(bytes: &[u8]) -> Result<Vec<Named>, String> {
    let (doc, bin): (Value, Option<&[u8]>) = if bytes.starts_with(b"glTF") {
        let jlen = u32::from_le_bytes(bytes.get(12..16).ok_or("the GLB is truncated")?.try_into().expect("4")) as usize;
        let doc = serde_json::from_slice(bytes.get(20..20 + jlen).ok_or("the GLB is truncated")?).map_err(|e| format!("the GLB JSON is broken: {e}"))?;
        let rest = 20 + jlen;
        let bin = (bytes.len() >= rest + 8).then(|| {
            let blen = u32::from_le_bytes(bytes[rest..rest + 4].try_into().expect("4")) as usize;
            &bytes[rest + 8..(rest + 8 + blen).min(bytes.len())]
        });
        (doc, bin)
    } else {
        (serde_json::from_slice(bytes).map_err(|e| format!("the glTF JSON is broken: {e}"))?, None)
    };
    let buffers = buffer_bytes(&doc, bin)?;
    let scene = doc["scene"].as_u64().unwrap_or(0) as usize;
    let roots: Vec<usize> = doc["scenes"][scene]["nodes"].as_array().map(|a| a.iter().filter_map(|x| x.as_u64().map(|v| v as usize)).collect()).unwrap_or_else(|| (0..doc["nodes"].as_array().map(|a| a.len()).unwrap_or(0)).collect());
    let mut stack: Vec<(usize, Mat4)> = roots.into_iter().map(|n| (n, Mat4::IDENTITY)).collect();
    let mut out = Vec::new();
    while let Some((n, parent)) = stack.pop() {
        let node = &doc["nodes"][n];
        let world = parent * local(node);
        if let Some(children) = node["children"].as_array() {
            for c in children.iter().filter_map(|x| x.as_u64()) {
                stack.push((c as usize, world));
            }
        }
        let Some(mi) = node["mesh"].as_u64() else { continue };
        let mesh = &doc["meshes"][mi as usize];
        let mut positions: Vec<[f32; 3]> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        let mut colors: Vec<[f32; 3]> = Vec::new();
        let mut any_color = false;
        for prim in mesh["primitives"].as_array().cloned().unwrap_or_default() {
            if prim["mode"].as_u64().unwrap_or(4) != 4 {
                continue;
            }
            let Some(pa) = prim["attributes"]["POSITION"].as_u64() else { continue };
            let p = accessor(&doc, &buffers, pa as usize)?;
            let base = positions.len() as u32;
            for c in p.chunks_exact(3) {
                positions.push(world.transform_point3(Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32)).to_array());
            }
            match prim["attributes"]["COLOR_0"].as_u64() {
                Some(ca) => {
                    let acc = &doc["accessors"][ca as usize];
                    let comps = if acc["type"] == "VEC4" { 4 } else { 3 };
                    let scale = match (acc["normalized"].as_bool().unwrap_or(false), acc["componentType"].as_u64()) {
                        (true, Some(5121)) => 255.0,
                        (true, Some(5123)) => 65535.0,
                        _ => 1.0,
                    };
                    let vals = accessor(&doc, &buffers, ca as usize)?;
                    colors.extend(vals.chunks_exact(comps).map(|c| [(c[0] / scale) as f32, (c[1] / scale) as f32, (c[2] / scale) as f32]));
                    any_color = true;
                }
                None => colors.extend(std::iter::repeat([0.8f32; 3]).take(p.len() / 3)),
            }
            let count = (p.len() / 3) as u32;
            match prim["indices"].as_u64() {
                Some(ia) => indices.extend(accessor(&doc, &buffers, ia as usize)?.into_iter().map(|i| base + i as u32)),
                None => indices.extend(base..base + count),
            }
        }
        if world.determinant() < 0.0 {
            for t in indices.chunks_exact_mut(3) {
                t.swap(1, 2);
            }
        }
        if indices.iter().any(|&i| i as usize >= positions.len()) {
            return Err("a glTF primitive index points past its vertices".into());
        }
        if !indices.is_empty() {
            let name = node["name"].as_str().or(mesh["name"].as_str()).unwrap_or("Imported").to_string();
            let mut mesh = Mesh::new(positions, indices);
            if any_color && colors.len() == mesh.vertex_count() {
                mesh.paint = colors.into_iter().map(crate::bin::with_color).collect();
            }
            out.push((name, mesh));
        }
    }
    if out.is_empty() {
        return Err("the glTF has no triangle meshes".into());
    }
    Ok(out)
}
