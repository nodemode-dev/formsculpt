use std::path::Path;
use std::time::Instant;

use fsc_core::Maps;
use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

pub(crate) fn unwrap(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let started = Instant::now();
    let object = s.scene.get_mut(id).expect("checked");
    let result = fsc_paint::unwrap(&object.mesh, 4.0 / 1024.0);
    let mut mesh = object.mesh.clone();
    mesh.uvs = result.uvs;
    let mask = object.mask.clone();
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    object.maps = None;
    let after = summary(object);
    s.history.push(Edit::Topology { label: "Unwrap UVs".into(), object: id, mesh: old_mesh, mask: old_mask });
    Ok(json!({"object": after, "charts": result.charts, "coverage": result.coverage, "ms": started.elapsed().as_millis()}))
}

pub(crate) fn bake_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "The object to bake onto (it needs UVs from uv_unwrap); the active object when omitted."},
            "size": {"type": "integer", "enum": [256, 512, 1024, 2048, 4096], "default": 1024, "description": "Texture width and height in pixels."},
            "source_id": {"type": "integer", "minimum": 1, "description": "Bake detail from this object (a high-resolution sculpt) instead; by default the object's highest subdivision level, or the object itself."},
            "path": {"type": "string", "minLength": 1, "description": "A folder to also write NAME_color.png, NAME_roughness_metal.png, NAME_normal.png and NAME_displacement.exr into."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn bake(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let size = args.get("size").and_then(Value::as_u64).unwrap_or(1024) as usize;
    if ![256, 512, 1024, 2048, 4096].contains(&size) {
        return Err(bad("size must be 256, 512, 1024, 2048 or 4096"));
    }
    let started = Instant::now();
    let object = s.scene.get(id).expect("checked");
    if object.mesh.uvs.is_empty() {
        return Err(CommandError::new("no_uvs", "the object has no UVs; run uv_unwrap first (UVs are cleared by topology changes)"));
    }
    let source_id = args.get("source_id").and_then(Value::as_u64).map(|x| x as u32);
    let high;
    let (source_mesh, source_bvh, source_name) = match source_id {
        Some(sid) => {
            let o = s.scene.get(sid).ok_or_else(|| CommandError::new("not_found", format!("no object {sid}")))?;
            (&o.mesh, &o.bvh, o.name.clone())
        }
        None => match object.levels() {
            (cur, n) if cur + 1 < n => {
                let mut copy = object.clone();
                copy.set_level(n - 1).map_err(|e| CommandError::new("bad_level", e))?;
                high = copy;
                (&high.mesh, &high.bvh, format!("level {}", n - 1))
            }
            _ => (&object.mesh, &object.bvh, "itself".to_string()),
        },
    };
    let max_dist = object.mesh.bounds().extent().length() * 0.05;
    let ex = fsc_paint::expand(&object.mesh);
    let baked = fsc_paint::bake(&object.mesh, &ex, source_mesh, source_bvh, size, max_dist, 6);
    let png = |px: &[[u8; 4]]| fsc_paint::png_rgba(size, px).map_err(|e| CommandError::new("encode_failed", e));
    let maps = Maps {
        size,
        color_png: png(&baked.color)?,
        mr_png: png(&baked.mr)?,
        normal_png: png(&baked.normal)?,
        displacement_exr: fsc_paint::exr_float(size, &baked.displacement),
        source: source_name.clone(),
    };
    let (lo, hi) = baked.displacement.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), &d| (a.min(d), b.max(d)));
    let mut written = Vec::new();
    if let Some(dir) = args.get("path").and_then(Value::as_str) {
        let stem: String = object.name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
        for (suffix, bytes) in [("color.png", &maps.color_png), ("roughness_metal.png", &maps.mr_png), ("normal.png", &maps.normal_png), ("displacement.exr", &maps.displacement_exr)] {
            let file = Path::new(dir).join(format!("{stem}_{suffix}"));
            std::fs::write(&file, bytes).map_err(|e| CommandError::new("io_failed", format!("could not write {}: {e}", file.display())))?;
            written.push(file.display().to_string());
        }
    }
    let texels = size * size;
    let object = s.scene.get_mut(id).expect("checked");
    object.maps = Some(maps);
    object.revision += 1;
    Ok(json!({
        "id": id,
        "size": size,
        "source": source_name,
        "covered": baked.covered as f32 / texels as f32,
        "hit": if baked.covered > 0 { baked.hits as f32 / baked.covered as f32 } else { 0.0 },
        "displacement_range": [lo, hi],
        "files": written,
        "ms": started.elapsed().as_millis(),
    }))
}
