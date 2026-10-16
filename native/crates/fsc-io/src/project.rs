use std::io::{Cursor, Read, Write};

use fsc_core::{Blend, Layer, LayerPaint, Layers, Level, Mesh, Multires, Object, Paint, PaintLayer, PaintLayers, Scene, SelectionSet, Stencil, COVER, PAINT_LEN};
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
            "folder": o.folder,
            "material": {"subsurface": o.material.subsurface, "subsurface_color": o.material.subsurface_color, "subsurface_radius": o.material.subsurface_radius, "transmission": o.material.transmission, "ior": o.material.ior},
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
        if o.mesh.has_groups() {
            entry["groups"] = json!(w.put(&format!("{dir}/groups.u32"), &o.mesh.groups_bytes())?);
        }
        if o.mesh.has_hidden() {
            entry["hidden"] = json!(w.put(&format!("{dir}/hidden.u8"), &o.mesh.hidden_bytes())?);
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
            if !m.emissive_png.is_empty() {
                entry["maps"]["emissive"] = json!(w.put(&format!("{dir}/maps/emissive.png"), &m.emissive_png)?);
            }
        }
        if !o.tile_maps.is_empty() {
            let mut tiles = Vec::new();
            for (tile, m) in &o.tile_maps {
                let mut t = json!({
                    "tile": tile,
                    "size": m.size,
                    "source": m.source,
                    "color": w.put(&format!("{dir}/maps/{tile}/color.png"), &m.color_png)?,
                    "roughness_metal": w.put(&format!("{dir}/maps/{tile}/roughness_metal.png"), &m.mr_png)?,
                    "normal": w.put(&format!("{dir}/maps/{tile}/normal.png"), &m.normal_png)?,
                    "displacement": w.put(&format!("{dir}/maps/{tile}/displacement.exr"), &m.displacement_exr)?,
                });
                if !m.emissive_png.is_empty() {
                    t["emissive"] = json!(w.put(&format!("{dir}/maps/{tile}/emissive.png"), &m.emissive_png)?);
                }
                tiles.push(t);
            }
            entry["tile_maps"] = json!(tiles);
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
        if o.paint_layers.valid(o.mesh.vertex_count()) {
            // Paint layers: the base as five floats per vertex, each layer as six (colour, roughness, metalness,
            // coverage); `paint` above already holds the composite, so older readers still see the right colours.
            let base: Vec<f32> = o.paint_layers.base.iter().flatten().copied().collect();
            let mut list = Vec::new();
            for (k, l) in o.paint_layers.list.iter().enumerate() {
                let data: Vec<f32> = l.data.iter().flatten().copied().collect();
                list.push(json!({"name": l.name, "blend": l.blend.name(), "opacity": l.opacity, "visible": l.visible, "data": w.put(&format!("{dir}/paintlayer{k}.f32"), &f32s(&data))?}));
            }
            entry["paint_layers"] = json!({"active": o.paint_layers.active, "base": w.put(&format!("{dir}/paintbase.f32"), &f32s(&base))?, "list": list});
        }
        if !o.selection_sets.is_empty() {
            let mut sets = Vec::new();
            for (k, set) in o.selection_sets.iter().enumerate() {
                let sd = format!("{dir}/sets/{k}");
                let mut se = json!({"name": set.name, "vertices": set.vertices, "faces": set.faces});
                if let Some(g) = &set.groups {
                    se["groups"] = json!(w.put(&format!("{sd}/groups.u32"), &u32s(g))?);
                }
                if let Some(m) = &set.mask {
                    se["mask"] = json!(w.put(&format!("{sd}/mask.f32"), &f32s(m))?);
                }
                if let Some(p) = &set.paint {
                    let flat: Vec<f32> = p.iter().flatten().copied().collect();
                    se["paint"] = json!(w.put(&format!("{sd}/paint.f32"), &f32s(&flat))?);
                }
                if let Some(st) = &set.strengths {
                    se["strengths"] = json!(st);
                }
                if let Some(u) = &set.uvs {
                    let flat: Vec<f32> = u.iter().flatten().copied().collect();
                    se["uvs"] = json!(w.put(&format!("{sd}/uvs.f32"), &f32s(&flat))?);
                }
                sets.push(se);
            }
            entry["selection_sets"] = json!(sets);
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
        paint_floats(&self.f32(v)?, n).ok_or_else(|| "a paint buffer does not match its mesh".into())
    }
}

/// Paint from a flat buffer of `n` vertices: seven floats each, or five from projects before 0.26.0 (opaque, no glow).
fn paint_floats(flat: &[f32], n: usize) -> Option<Vec<Paint>> {
    if flat.len() == n * PAINT_LEN {
        Some(flat.chunks_exact(PAINT_LEN).map(|c| std::array::from_fn(|k| c[k])).collect())
    } else if flat.len() == n * 5 {
        Some(flat.chunks_exact(5).map(fsc_core::paint_from5).collect())
    } else {
        None
    }
}

/// A paint layer's data: the paint floats and a coverage per vertex, in either layout `paint_floats` reads.
fn layer_floats(flat: &[f32], n: usize) -> Option<Vec<LayerPaint>> {
    let stride = if flat.len() == n * (PAINT_LEN + 1) { PAINT_LEN } else if flat.len() == n * 6 { 5 } else { return None };
    Some(
        flat.chunks_exact(stride + 1)
            .map(|c| {
                let mut out = [0.0; PAINT_LEN + 1];
                out[..PAINT_LEN].copy_from_slice(&if stride == PAINT_LEN { std::array::from_fn(|k| c[k]) } else { fsc_core::paint_from5(c) });
                out[COVER] = c[stride];
                out
            })
            .collect(),
    )
}

fn checked(positions: Vec<[f32; 3]>, indices: Vec<u32>) -> Result<Mesh, String> {
    if !indices.len().is_multiple_of(3) || indices.iter().any(|&i| i as usize >= positions.len()) {
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
        if e["groups"].is_string() {
            let groups = r.u32(&e["groups"])?;
            if groups.len() != mesh.triangle_count() {
                return Err("a face group buffer does not match its mesh".into());
            }
            mesh.groups = groups;
        }
        if e["hidden"].is_string() {
            let hidden = r.raw(&e["hidden"])?;
            if hidden.len() != mesh.triangle_count() {
                return Err("a hidden-face buffer does not match its mesh".into());
            }
            mesh.hidden = hidden.into_iter().map(|b| b != 0).collect();
        }
        let maps = match e.get("maps").filter(|m| m.is_object()) {
            Some(m) => Some(fsc_core::Maps {
                size: m["size"].as_u64().unwrap_or(0) as usize,
                source: m["source"].as_str().unwrap_or("").to_string(),
                color_png: r.raw(&m["color"])?,
                mr_png: r.raw(&m["roughness_metal"])?,
                normal_png: r.raw(&m["normal"])?,
                displacement_exr: r.raw(&m["displacement"])?,
                emissive_png: if m["emissive"].is_string() { r.raw(&m["emissive"])? } else { Vec::new() },
            }),
            None => None,
        };
        let mut o = Object::new(id, e["name"].as_str().unwrap_or("Object").to_string(), mesh);
        o.visible = e["visible"].as_bool().unwrap_or(true);
        o.folder = e["folder"].as_str().unwrap_or("").chars().take(80).collect();
        // Projects from before 0.24.0 have no material; they open opaque.
        let m = &e["material"];
        let f = |k: &str, d: f32, lo: f32, hi: f32| m[k].as_f64().map(|v| (v as f32).clamp(lo, hi)).unwrap_or(d);
        let base = fsc_core::Material::default();
        let color: Vec<f32> = m["subsurface_color"].as_array().map(|a| a.iter().filter_map(|x| x.as_f64()).map(|x| (x as f32).clamp(0.0, 1.0)).collect()).unwrap_or_default();
        o.material = fsc_core::Material {
            subsurface: f("subsurface", base.subsurface, 0.0, 1.0),
            subsurface_color: if color.len() == 3 { [color[0], color[1], color[2]] } else { base.subsurface_color },
            subsurface_radius: f("subsurface_radius", base.subsurface_radius, 1e-4, 1000.0),
            transmission: f("transmission", base.transmission, 0.0, 1.0),
            ior: f("ior", base.ior, 1.0, 2.5),
        };
        o.maps = maps;
        for t in e.get("tile_maps").and_then(|t| t.as_array()).into_iter().flatten() {
            let Some(tile) = t["tile"].as_u64() else { continue };
            o.tile_maps.push((
                tile as u32,
                fsc_core::Maps {
                    size: t["size"].as_u64().unwrap_or(0) as usize,
                    source: t["source"].as_str().unwrap_or("").to_string(),
                    color_png: r.raw(&t["color"])?,
                    mr_png: r.raw(&t["roughness_metal"])?,
                    normal_png: r.raw(&t["normal"])?,
                    displacement_exr: r.raw(&t["displacement"])?,
                    emissive_png: if t["emissive"].is_string() { r.raw(&t["emissive"])? } else { Vec::new() },
                },
            ));
        }
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
        if let Some(pl) = e.get("paint_layers").filter(|l| l.is_object()) {
            let vc = o.mesh.vertex_count();
            let base = paint_floats(&r.f32(&pl["base"])?, vc).ok_or("the paint base does not match its object")?;
            let mut list = Vec::new();
            for le in pl["list"].as_array().cloned().unwrap_or_default() {
                let data = layer_floats(&r.f32(&le["data"])?, vc).ok_or("a paint layer does not match its object")?;
                list.push(PaintLayer {
                    name: le["name"].as_str().unwrap_or("Paint").chars().take(64).collect(),
                    blend: le["blend"].as_str().and_then(Blend::parse).unwrap_or(Blend::Normal),
                    opacity: le["opacity"].as_f64().unwrap_or(1.0).clamp(0.0, 1.0) as f32,
                    visible: le["visible"].as_bool().unwrap_or(true),
                    data,
                });
            }
            if !list.is_empty() {
                let active = pl["active"].as_u64().map(|a| a as usize).filter(|&a| a < list.len());
                o.paint_layers = PaintLayers { list, active, base };
                o.composite_paint(None);
            }
        }
        for se in e["selection_sets"].as_array().cloned().unwrap_or_default() {
            let broken = "a selection set does not match its recorded counts";
            let (vertices, faces) = (se["vertices"].as_u64().unwrap_or(0) as usize, se["faces"].as_u64().unwrap_or(0) as usize);
            let mut set = SelectionSet { name: se["name"].as_str().unwrap_or("Set").chars().take(64).collect(), vertices, faces, ..SelectionSet::default() };
            if se["groups"].is_string() {
                set.groups = Some(r.u32(&se["groups"])?).filter(|g| g.len() == faces);
                set.groups.as_ref().ok_or(broken)?;
            }
            if se["mask"].is_string() {
                set.mask = Some(r.f32(&se["mask"])?).filter(|m| m.len() == vertices);
                set.mask.as_ref().ok_or(broken)?;
            }
            if se["paint"].is_string() {
                set.paint = Some(r.paint(&se["paint"], vertices)?);
            }
            if let Some(st) = se["strengths"].as_array() {
                set.strengths = Some(st.iter().map(|x| x.as_f64().unwrap_or(1.0) as f32).collect());
            }
            if se["uvs"].is_string() {
                let flat = r.f32(&se["uvs"])?;
                if flat.len() != faces * 6 {
                    return Err(broken.into());
                }
                set.uvs = Some(flat.chunks_exact(2).map(|c| [c[0], c[1]]).collect());
            }
            o.selection_sets.push(set);
        }
        let at = scene.objects.len();
        scene.insert(at, o);
    }
    scene.active = manifest["active"].as_u64().map(|a| a as u32).filter(|a| scene.get(*a).is_some());
    Ok(scene)
}
