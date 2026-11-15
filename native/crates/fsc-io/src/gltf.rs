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

/// An object's UDIM split: the tile number of each triangle (0: outside every tile) and the maps baked per tile.
pub struct TileSplit<'a> {
    pub of_triangle: Vec<u32>,
    pub maps: Vec<(u32, &'a fsc_core::Maps)>,
}

pub struct GltfObject<'a> {
    pub name: &'a str,
    pub mesh: &'a Mesh,
    pub targets: Vec<Target<'a>>,
    pub maps: Option<&'a fsc_core::Maps>,
    /// When set and non-empty, the object exports one primitive and material per tile that has maps.
    pub tiles: Option<TileSplit<'a>>,
}

struct Prim<'a> {
    maps: Option<&'a fsc_core::Maps>,
    tile: Option<u32>,
    pos: usize,
    nor: usize,
    targets: Vec<usize>,
    color: Option<usize>,
    uv: Option<usize>,
    tangent: Option<usize>,
    indices: Vec<u32>,
}

fn pad(buf: &mut Vec<u8>, with: u8) {
    while !buf.len().is_multiple_of(4) {
        buf.push(with);
    }
}

/// A GLB: the document and its one buffer in a single file.
pub fn write_glb(objects: &[GltfObject]) -> Vec<u8> {
    let (doc, bin) = document(objects);
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

/// A glTF in its separate form: the JSON document, whose buffer points at `bin_name` beside it, and that buffer's
/// bytes (geometry, paint, layers and any baked maps, as in the GLB).
pub fn write_gltf(objects: &[GltfObject], bin_name: &str) -> (Vec<u8>, Vec<u8>) {
    let (mut doc, bin) = document(objects);
    doc["buffers"][0]["uri"] = json!(bin_name);
    (serde_json::to_vec_pretty(&doc).expect("json"), bin)
}

fn document(objects: &[GltfObject]) -> (Value, Vec<u8>) {
    let mut bin: Vec<u8> = Vec::new();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut meshes = Vec::new();
    let mut nodes = Vec::new();
    let mut images = Vec::new();
    let mut add = |bin: &mut Vec<u8>, data: &[f32], comps: usize, minmax: bool| -> usize {
        pad(bin, 0);
        let offset = bin.len();
        let (mut lo, mut hi) = (vec![f32::INFINITY; comps], vec![f32::NEG_INFINITY; comps]);
        for (i, x) in data.iter().enumerate() {
            bin.extend_from_slice(&x.to_le_bytes());
            lo[i % comps] = lo[i % comps].min(*x);
            hi[i % comps] = hi[i % comps].max(*x);
        }
        views.push(json!({"buffer": 0, "byteOffset": offset, "byteLength": data.len() * 4, "target": 34962}));
        let ty = ["SCALAR", "VEC2", "VEC3", "VEC4"][comps - 1];
        let mut acc = json!({"bufferView": views.len() - 1, "componentType": 5126, "count": data.len() / comps, "type": ty});
        if minmax {
            acc["min"] = json!(lo);
            acc["max"] = json!(hi);
        }
        accessors.push(acc);
        accessors.len() - 1
    };
    let mut prepared: Vec<Vec<Prim>> = Vec::new();
    for o in objects {
        let split = o.tiles.as_ref().filter(|t| !t.maps.is_empty() && o.mesh.uvs.len() == o.mesh.indices.len() && !o.mesh.uvs.is_empty() && t.of_triangle.len() == o.mesh.triangle_count());
        let mut variants: Vec<(Option<Mesh>, Option<&fsc_core::Maps>, Option<u32>)> = Vec::new();
        match split {
            None => variants.push((None, o.maps, None)),
            Some(t) => {
                let mut parts: Vec<Vec<usize>> = vec![Vec::new(); t.maps.len() + 1];
                for (tri, tile) in t.of_triangle.iter().enumerate() {
                    parts[t.maps.iter().position(|(n, _)| n == tile).unwrap_or(t.maps.len())].push(tri);
                }
                for (k, tris) in parts.iter().enumerate() {
                    if tris.is_empty() {
                        continue;
                    }
                    let tile = t.maps.get(k).map(|(n, _)| *n);
                    let (du, dv) = tile.map_or((0.0, 0.0), |n| (((n.saturating_sub(1001)) % 10) as f32, ((n.saturating_sub(1001)) / 10) as f32));
                    let mut sub = o.mesh.clone();
                    sub.indices = tris.iter().flat_map(|&t| o.mesh.indices[3 * t..3 * t + 3].iter().copied()).collect();
                    sub.uvs = tris.iter().flat_map(|&t| o.mesh.uvs[3 * t..3 * t + 3].iter().map(|p| [p[0] - du, p[1] - dv])).collect();
                    sub.groups = Vec::new();
                    sub.hidden = Vec::new();
                    variants.push((Some(sub), t.maps.get(k).map(|(_, m)| *m).or(o.maps), tile));
                }
            }
        }
        let mut prims = Vec::new();
        for (sub, maps, tile) in variants {
            let m: &Mesh = sub.as_ref().unwrap_or(o.mesh);
            let ex = (!m.uvs.is_empty()).then(|| fsc_paint::expand(m));
            let src: Vec<usize> = match &ex {
                Some(e) => e.source.iter().map(|&s| s as usize).collect(),
                None => (0..m.vertex_count()).collect(),
            };
            let base: Vec<f32> = src
                .iter()
                .flat_map(|&v| {
                    let mut q = Vec3::from_array(m.positions[v]);
                    for t in &o.targets {
                        q -= Vec3::from_array(t.delta[v]) * t.weight;
                    }
                    q.to_array()
                })
                .collect();
            let pos = add(&mut bin, &base, 3, true);
            let normals: Vec<f32> = src.iter().flat_map(|&v| m.normals[v]).collect();
            let nor = add(&mut bin, &normals, 3, false);
            let targets: Vec<usize> = o
                .targets
                .iter()
                .map(|t| {
                    let d: Vec<f32> = src.iter().flat_map(|&v| t.delta[v]).collect();
                    add(&mut bin, &d, 3, true)
                })
                .collect();
            // Vertex colours carry opacity as alpha when any of it is see-through.
            let color = (m.has_paint() && maps.is_none()).then(|| {
                if m.paint.iter().any(|p| p[5] < 0.999) {
                    let rgba: Vec<f32> = src.iter().flat_map(|&v| [m.paint[v][0], m.paint[v][1], m.paint[v][2], m.paint[v][5]]).collect();
                    add(&mut bin, &rgba, 4, false)
                } else {
                    let rgb: Vec<f32> = src.iter().flat_map(|&v| [m.paint[v][0], m.paint[v][1], m.paint[v][2]]).collect();
                    add(&mut bin, &rgb, 3, false)
                }
            });
            let uv = ex.as_ref().map(|e| {
                let flat: Vec<f32> = e.uv.iter().flatten().copied().collect();
                add(&mut bin, &flat, 2, split.is_some())
            });
            let tangent = ex.as_ref().filter(|_| maps.is_some()).map(|e| {
                let flat: Vec<f32> = e.tangent.iter().flatten().copied().collect();
                add(&mut bin, &flat, 4, false)
            });
            let indices: Vec<u32> = match &ex {
                Some(e) => e.corner.clone(),
                None => m.indices.clone(),
            };
            prims.push(Prim { maps, tile, pos, nor, targets, color, uv, tangent, indices });
        }
        prepared.push(prims);
    }
    let mut materials = Vec::new();
    let mut textures = Vec::new();
    for (k, o) in objects.iter().enumerate() {
        let mut primitives = Vec::new();
        for Prim { maps, tile, pos, nor, targets, color, uv, tangent, indices } in &prepared[k] {
        pad(&mut bin, 0);
        let offset = bin.len();
        for i in indices {
            bin.extend_from_slice(&i.to_le_bytes());
        }
        views.push(json!({"buffer": 0, "byteOffset": offset, "byteLength": indices.len() * 4, "target": 34963}));
        accessors.push(json!({"bufferView": views.len() - 1, "componentType": 5125, "count": indices.len(), "type": "SCALAR"}));
        let mut prim = json!({"attributes": {"POSITION": pos, "NORMAL": nor}, "indices": accessors.len() - 1, "mode": 4, "material": materials.len()});
        if let Some(u) = uv {
            prim["attributes"]["TEXCOORD_0"] = json!(u);
        }
        if let Some(t) = tangent {
            prim["attributes"]["TANGENT"] = json!(t);
        }
        let n = o.mesh.vertex_count().max(1) as f32;
        let (rough, metal) = if o.mesh.has_paint() {
            (o.mesh.paint.iter().map(|p| p[3]).sum::<f32>() / n, o.mesh.paint.iter().map(|p| p[4]).sum::<f32>() / n)
        } else {
            (fsc_core::DEFAULT_PAINT[3], fsc_core::DEFAULT_PAINT[4])
        };
        let tint = if color.is_some() { [1.0, 1.0, 1.0, 1.0] } else { [0.8, 0.8, 0.8, 1.0] };
        let label = match tile {
            Some(t) => format!("{}_{t}", o.name),
            None => o.name.to_string(),
        };
        let mut material = json!({"name": label, "pbrMetallicRoughness": {"baseColorFactor": tint, "metallicFactor": metal, "roughnessFactor": rough}});
        if let (Some(maps), Some(_)) = (*maps, uv) {
            let mut texture = |png: &[u8]| -> usize {
                pad(&mut bin, 0);
                let offset = bin.len();
                bin.extend_from_slice(png);
                views.push(json!({"buffer": 0, "byteOffset": offset, "byteLength": png.len()}));
                images.push(json!({"bufferView": views.len() - 1, "mimeType": "image/png"}));
                textures.push(json!({"source": images.len() - 1, "sampler": 0}));
                textures.len() - 1
            };
            let (c, mr) = (texture(&maps.color_png), texture(&maps.mr_png));
            let nm = (!maps.normal_png.is_empty()).then(|| texture(&maps.normal_png));
            material = json!({"name": label, "pbrMetallicRoughness": {"baseColorTexture": {"index": c}, "metallicRoughnessTexture": {"index": mr}, "metallicFactor": 1.0, "roughnessFactor": 1.0}});
            if let Some(nm) = nm {
                material["normalTexture"] = json!({"index": nm});
            }
            if !maps.emissive_png.is_empty() {
                material["emissiveTexture"] = json!({"index": texture(&maps.emissive_png)});
                material["emissiveFactor"] = json!([1.0, 1.0, 1.0]);
            }
        } else if o.mesh.has_paint() {
            // Without baked maps glow cannot vary across the surface in glTF: the mean glow colour is the factor.
            let glow = o.mesh.paint.iter().fold([0.0f32; 3], |a, p| [a[0] + p[0] * p[6], a[1] + p[1] * p[6], a[2] + p[2] * p[6]]).map(|x| x / n);
            if glow.iter().any(|&g| g > 1e-3) {
                material["emissiveFactor"] = json!(glow);
            }
        }
        let from_texture = maps.is_some_and(|m| m.source == "texture");
        if maps.is_some_and(|m| m.translucent) || (!from_texture && o.mesh.has_paint() && o.mesh.paint.iter().any(|p| p[5] < 0.999)) {
            material["alphaMode"] = json!("BLEND");
        }
        materials.push(material);
        if let Some(c) = color {
            prim["attributes"]["COLOR_0"] = json!(c);
        }
        if !targets.is_empty() {
            prim["targets"] = json!(targets.iter().map(|t| json!({"POSITION": t})).collect::<Vec<_>>());
        }
        primitives.push(prim);
        }
        let mut mesh = json!({"name": o.name, "primitives": []});
        if !o.targets.is_empty() {
            mesh["weights"] = json!(o.targets.iter().map(|t| t.weight).collect::<Vec<_>>());
            mesh["extras"] = json!({"targetNames": o.targets.iter().map(|t| t.name).collect::<Vec<_>>()});
        }
        mesh["primitives"] = json!(primitives);
        meshes.push(mesh);
        nodes.push(json!({"name": o.name, "mesh": k}));
    }
    pad(&mut bin, 0);
    let mut doc = json!({
        "asset": {"version": "2.0", "generator": format!("formsculpt {}", fsc_core::engine_version())},
        "scene": 0,
        "scenes": [{"nodes": (0..objects.len()).collect::<Vec<_>>()}],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
        "buffers": [{"byteLength": bin.len()}],
        "bufferViews": views,
        "accessors": accessors,
    });
    if !images.is_empty() {
        doc["images"] = json!(images);
        doc["textures"] = json!(textures);
        doc["samplers"] = json!([{"magFilter": 9729, "minFilter": 9987, "wrapS": 33071, "wrapT": 33071}]);
    }
    (doc, bin)
}

/// Reads a file a glTF refers to by URI (a .bin beside it); the importer passes one that looks next to the .gltf.
pub type Resolve<'a> = &'a dyn Fn(&str) -> Result<Vec<u8>, String>;

fn buffer_bytes(doc: &Value, glb_bin: Option<&[u8]>, resolve: Resolve) -> Result<Vec<Vec<u8>>, String> {
    let mut out = Vec::new();
    for (k, b) in doc["buffers"].as_array().cloned().unwrap_or_default().iter().enumerate() {
        match b["uri"].as_str() {
            None if k == 0 => out.push(glb_bin.ok_or("buffer 0 has no data")?.to_vec()),
            Some(uri) if uri.starts_with("data:") => {
                let data = uri.split(",").nth(1).ok_or("a data URI has no data")?;
                out.push(base64::engine::general_purpose::STANDARD.decode(data).map_err(|e| format!("a data URI is not base64: {e}"))?);
            }
            Some(uri) => {
                let data = resolve(uri)?;
                let need = b["byteLength"].as_u64().unwrap_or(0) as usize;
                if data.len() < need {
                    return Err(format!("{uri} is shorter than the glTF says ({} of {need} bytes)", data.len()));
                }
                out.push(data);
            }
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
    let Some(view_index) = a["bufferView"].as_u64() else { return Err("an accessor keeps its data outside the buffers (sparse or compressed, such as Draco), which is not read".into()) };
    let view = &doc["bufferViews"][view_index as usize];
    let buf = buffers.get(view["buffer"].as_u64().unwrap_or(0) as usize).ok_or("a view points at a missing buffer")?;
    let base = (view["byteOffset"].as_u64().unwrap_or(0) as usize).saturating_add(a["byteOffset"].as_u64().unwrap_or(0) as usize);
    let stride = view["byteStride"].as_u64().map(|s| s as usize).unwrap_or(size * comps);
    if stride < size * comps || base > buf.len() || count.saturating_sub(1) > buf.len() / stride {
        return Err("an accessor reads past its buffer".into());
    }
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
    if t.len() < 3 || r.len() < 4 || s.len() < 3 {
        return Mat4::IDENTITY;
    }
    Mat4::from_scale_rotation_translation(Vec3::new(s[0], s[1], s[2]), Quat::from_xyzw(r[0], r[1], r[2], r[3]), Vec3::new(t[0], t[1], t[2]))
}

pub fn read_gltf_with(bytes: &[u8], resolve: Resolve) -> Result<Vec<Named>, String> {
    let (doc, bin): (Value, Option<&[u8]>) = if bytes.starts_with(b"glTF") {
        let jlen = u32::from_le_bytes(bytes.get(12..16).ok_or("the GLB is truncated")?.try_into().expect("4")) as usize;
        let doc = serde_json::from_slice(bytes.get(20..20 + jlen).ok_or("the GLB is truncated")?).map_err(|e| format!("the GLB JSON is broken: {e}"))?;
        let rest = 20 + jlen;
        let bin = (bytes.len() >= rest + 8).then(|| {
            let blen = u32::from_le_bytes(bytes[rest..rest + 4].try_into().expect("4")) as usize;
            &bytes[rest + 8..(rest + 8).saturating_add(blen).min(bytes.len())]
        });
        (doc, bin)
    } else {
        (serde_json::from_slice(bytes).map_err(|e| format!("the glTF JSON is broken: {e}"))?, None)
    };
    let buffers = buffer_bytes(&doc, bin, resolve)?;
    let scene = doc["scene"].as_u64().unwrap_or(0) as usize;
    let roots: Vec<usize> = doc["scenes"][scene]["nodes"].as_array().map(|a| a.iter().filter_map(|x| x.as_u64().map(|v| v as usize)).collect()).unwrap_or_else(|| {
        let nodes = doc["nodes"].as_array().map_or(&[][..], |a| a.as_slice());
        let children: std::collections::HashSet<u64> = nodes.iter().filter_map(|n| n["children"].as_array()).flatten().filter_map(|c| c.as_u64()).collect();
        (0..nodes.len()).filter(|&i| !children.contains(&(i as u64))).collect()
    });
    let mut stack: Vec<(usize, Mat4)> = roots.into_iter().map(|n| (n, Mat4::IDENTITY)).collect();
    let mut out = Vec::new();
    let mut seen = vec![false; doc["nodes"].as_array().map_or(0, |a| a.len())];
    while let Some((n, parent)) = stack.pop() {
        if n >= seen.len() || std::mem::replace(&mut seen[n], true) {
            continue;
        }
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
        // Colour and alpha (opacity) per vertex.
        let mut colors: Vec<[f32; 4]> = Vec::new();
        let mut any_color = false;
        for prim in mesh["primitives"].as_array().cloned().unwrap_or_default() {
            if prim["mode"].as_u64().unwrap_or(4) != 4 {
                continue;
            }
            let Some(pa) = prim["attributes"]["POSITION"].as_u64() else { continue };
            let p = accessor(&doc, &buffers, pa as usize)?;
            let base = positions.len() as u32;
            for c in p.as_chunks::<3>().0 {
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
                    colors.extend(vals.chunks_exact(comps).map(|c| [(c[0] / scale) as f32, (c[1] / scale) as f32, (c[2] / scale) as f32, if comps == 4 { (c[3] / scale) as f32 } else { 1.0 }]));
                    any_color = true;
                }
                None => colors.extend(std::iter::repeat_n([0.8f32, 0.8, 0.8, 1.0], p.len() / 3)),
            }
            let count = (p.len() / 3) as u32;
            match prim["indices"].as_u64() {
                Some(ia) => indices.extend(accessor(&doc, &buffers, ia as usize)?.into_iter().map(|i| base.saturating_add(i as u32))),
                None => indices.extend(base..base + count),
            }
        }
        if world.determinant() < 0.0 {
            for t in indices.as_chunks_mut::<3>().0 {
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
                mesh.paint = colors
                    .into_iter()
                    .map(|c| {
                        let mut p = crate::bin::with_color([c[0], c[1], c[2]]);
                        p[5] = c[3].clamp(0.0, 1.0);
                        p
                    })
                    .collect();
            }
            out.push((name, mesh));
        }
    }
    if out.is_empty() {
        return Err("the glTF has no triangle meshes".into());
    }
    Ok(out)
}
