use std::io::{Cursor, Read, Write};

use fsc_core::{Layer, Layers, Level, Mesh, Multires, Object, Paint, Scene, Stencil};
use serde_json::{json, Value};

use crate::bin::{f32s, read_f32s, read_u32s, read_vec3s, u32s, vec3s};

const FORMAT: &str = "formsculptor-project";

struct Writer {
    zip: zip::ZipWriter<Cursor<Vec<u8>>>,
}

impl Writer {
    fn put(&mut self, name: &str, data: &[u8]) -> Result<String, String> {
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated).large_file(data.len() > 0xFFFF_0000);
        self.zip.start_file(name, opts).map_err(|e| e.to_string())?;
        self.zip.write_all(data).map_err(|e| e.to_string())?;
        Ok(name.to_string())
    }
}

pub fn save(scene: &Scene) -> Result<Vec<u8>, String> {
    let mut w = Writer { zip: zip::ZipWriter::new(Cursor::new(Vec::new())) };
    let mut objects = Vec::new();
    for o in &scene.objects {
        let dir = format!("objects/{}", o.id);
        let mut entry = json!({
            "id": o.id,
            "name": o.name,
            "visible": o.visible,
            "positions": w.put(&format!("{dir}/positions.f32"), &vec3s(&o.mesh.positions))?,
            "indices": w.put(&format!("{dir}/indices.u32"), &u32s(&o.mesh.indices))?,
            "mask": w.put(&format!("{dir}/mask.f32"), &f32s(&o.mask))?,
        });
        if o.mesh.has_paint() {
            entry["paint"] = json!(w.put(&format!("{dir}/paint.f32"), &o.mesh.paint_bytes())?);
        }
        if !o.mesh.uvs.is_empty() {
            let flat: Vec<f32> = o.mesh.uvs.iter().flatten().copied().collect();
            entry["uvs"] = json!(w.put(&format!("{dir}/uvs.f32"), &f32s(&flat))?);
        }
        if let Some(m) = &o.maps {
            entry["maps"] = json!({
                "size": m.size,
                "source": m.source,
                "color": w.put(&format!("{dir}/maps/color.png"), &m.color_png)?,
                "roughness_metal": w.put(&format!("{dir}/maps/roughness_metal.png"), &m.mr_png)?,
                "normal": w.put(&format!("{dir}/maps/normal.png"), &m.normal_png)?,
                "displacement": w.put(&format!("{dir}/maps/displacement.exr"), &m.displacement_exr)?,
            });
        }
        if let Some(m) = &o.multires {
            let mut levels = Vec::new();
            for (k, l) in m.levels.iter().enumerate() {
                let ld = format!("{dir}/level{k}");
                let mut le = json!({"vertices": l.vertices, "triangles": l.triangles});
                if k != m.current {
                    le["positions"] = json!(w.put(&format!("{ld}/positions.f32"), &vec3s(&l.mesh.positions))?);
                    le["indices"] = json!(w.put(&format!("{ld}/indices.u32"), &u32s(&l.mesh.indices))?);
                    if l.mesh.has_paint() {
                        le["paint"] = json!(w.put(&format!("{ld}/paint.f32"), &l.mesh.paint_bytes())?);
                    }
                }
                if !l.evens.is_empty() {
                    le["evens"] = json!(w.put(&format!("{ld}/evens.u32"), &u32s(&l.evens))?);
                }
                if !l.stencil.is_empty() {
                    le["stencil"] = json!({
                        "offsets": w.put(&format!("{ld}/stencil_offsets.u32"), &u32s(&l.stencil.offsets))?,
                        "sources": w.put(&format!("{ld}/stencil_sources.u32"), &u32s(&l.stencil.sources))?,
                        "weights": w.put(&format!("{ld}/stencil_weights.f32"), &f32s(&l.stencil.weights))?,
                    });
                }
                if !l.detail.is_empty() {
                    le["detail"] = json!(w.put(&format!("{ld}/detail.f32"), &vec3s(&l.detail))?);
                }
                levels.push(le);
            }
            entry["multires"] = json!({"current": m.current, "levels": levels});
        }
        if !o.layers.list.is_empty() {
            let mut list = Vec::new();
            for (k, l) in o.layers.list.iter().enumerate() {
                list.push(json!({"name": l.name, "strength": l.strength, "delta": w.put(&format!("{dir}/layer{k}.f32"), &vec3s(&l.delta))?}));
            }
            entry["layers"] = json!({"active": o.layers.active, "list": list});
        }
        objects.push(entry);
    }
    let manifest = json!({"format": FORMAT, "version": 1, "engine": fsc_core::engine_version(), "active": scene.active, "objects": objects});
    w.put("manifest.json", &serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?)?;
    Ok(w.zip.finish().map_err(|e| e.to_string())?.into_inner())
}

struct Reader {
    zip: zip::ZipArchive<Cursor<Vec<u8>>>,
}

impl Reader {
    fn raw(&mut self, v: &Value) -> Result<Vec<u8>, String> {
        let name = v.as_str().ok_or("the manifest names a missing buffer")?;
        let mut f = self.zip.by_name(name).map_err(|_| format!("the project is missing {name}"))?;
        let mut out = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut out).map_err(|e| e.to_string())?;
        Ok(out)
    }
    fn vec3(&mut self, v: &Value) -> Result<Vec<[f32; 3]>, String> {
        read_vec3s(&self.raw(v)?)
    }
    fn u32(&mut self, v: &Value) -> Result<Vec<u32>, String> {
        read_u32s(&self.raw(v)?)
    }
    fn f32(&mut self, v: &Value) -> Result<Vec<f32>, String> {
        read_f32s(&self.raw(v)?)
    }
    fn paint(&mut self, v: &Value, n: usize) -> Result<Vec<Paint>, String> {
        if !v.is_string() {
            return Ok(Vec::new());
        }
        let flat = self.f32(v)?;
        if flat.len() != n * 5 {
            return Err("a paint buffer does not match its mesh".into());
        }
        Ok(flat.chunks_exact(5).map(|c| [c[0], c[1], c[2], c[3], c[4]]).collect())
    }
}

fn checked(positions: Vec<[f32; 3]>, indices: Vec<u32>) -> Result<Mesh, String> {
    if indices.len() % 3 != 0 || indices.iter().any(|&i| i as usize >= positions.len()) {
        return Err("a mesh in the project has broken indices".into());
    }
    Ok(Mesh::new(positions, indices))
}

pub fn open(bytes: Vec<u8>) -> Result<Scene, String> {
    let zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|e| format!("not a formsculptor project (not a zip): {e}"))?;
    let mut r = Reader { zip };
    let manifest: Value = serde_json::from_slice(&r.raw(&json!("manifest.json"))?).map_err(|e| format!("the manifest is broken: {e}"))?;
    if manifest["format"] != FORMAT {
        return Err("not a formsculptor project (wrong manifest format)".into());
    }
    if manifest["version"].as_u64() != Some(1) {
        return Err("this project was saved by a newer formsculptor".into());
    }
    let mut scene = Scene::default();
    for e in manifest["objects"].as_array().cloned().unwrap_or_default() {
        let id = e["id"].as_u64().ok_or("an object has no id")? as u32;
        let mut mesh = checked(r.vec3(&e["positions"])?, r.u32(&e["indices"])?)?;
        mesh.paint = r.paint(&e["paint"], mesh.vertex_count())?;
        if e["uvs"].is_string() {
            let flat = r.f32(&e["uvs"])?;
            if flat.len() != mesh.indices.len() * 2 {
                return Err("a UV buffer does not match its mesh".into());
            }
            mesh.uvs = flat.chunks_exact(2).map(|c| [c[0], c[1]]).collect();
        }
        let maps = match e.get("maps").filter(|m| m.is_object()) {
            Some(m) => Some(fsc_core::Maps {
                size: m["size"].as_u64().unwrap_or(0) as usize,
                source: m["source"].as_str().unwrap_or("").to_string(),
                color_png: r.raw(&m["color"])?,
                mr_png: r.raw(&m["roughness_metal"])?,
                normal_png: r.raw(&m["normal"])?,
                displacement_exr: r.raw(&m["displacement"])?,
            }),
            None => None,
        };
        let mut o = Object::new(id, e["name"].as_str().unwrap_or("Object").to_string(), mesh);
        o.visible = e["visible"].as_bool().unwrap_or(true);
        o.maps = maps;
        let mask = r.f32(&e["mask"])?;
        if mask.len() == o.mesh.vertex_count() {
            o.mask = mask;
        }
        if let Some(m) = e.get("multires").filter(|m| m.is_object()) {
            let current = m["current"].as_u64().ok_or("levels have no current level")? as usize;
            let mut levels = Vec::new();
            for (k, le) in m["levels"].as_array().cloned().unwrap_or_default().iter().enumerate() {
                let mut level = Level { vertices: le["vertices"].as_u64().unwrap_or(0) as usize, triangles: le["triangles"].as_u64().unwrap_or(0) as usize, ..Level::default() };
                if k != current {
                    level.mesh = checked(r.vec3(&le["positions"])?, r.u32(&le["indices"])?)?;
                    level.mesh.paint = r.paint(&le["paint"], level.mesh.vertex_count())?;
                }
                if le["evens"].is_string() {
                    level.evens = r.u32(&le["evens"])?;
                }
                if le["stencil"].is_object() {
                    level.stencil = Stencil { offsets: r.u32(&le["stencil"]["offsets"])?, sources: r.u32(&le["stencil"]["sources"])?, weights: r.f32(&le["stencil"]["weights"])? };
                }
                if le["detail"].is_string() {
                    level.detail = r.vec3(&le["detail"])?;
                }
                levels.push(level);
            }
            if current >= levels.len() || levels[current].vertices != o.mesh.vertex_count() {
                return Err("the level stack does not match its object".into());
            }
            o.multires = Some(Multires { levels, current });
        }
        if let Some(l) = e.get("layers").filter(|l| l.is_object()) {
            let mut list = Vec::new();
            for le in l["list"].as_array().cloned().unwrap_or_default() {
                let delta = r.vec3(&le["delta"])?;
                if delta.len() != o.mesh.vertex_count() {
                    return Err("a layer does not match its object".into());
                }
                list.push(Layer { name: le["name"].as_str().unwrap_or("Layer").to_string(), strength: le["strength"].as_f64().unwrap_or(1.0) as f32, delta });
            }
            let active = l["active"].as_u64().map(|a| a as usize).filter(|&a| a < list.len());
            o.layers = Layers { list, active };
        }
        let at = scene.objects.len();
        scene.insert(at, o);
    }
    scene.active = manifest["active"].as_u64().map(|a| a as u32).filter(|a| scene.get(*a).is_some());
    Ok(scene)
}
