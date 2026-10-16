use std::path::Path;
use fsc_core::time::Instant;

use base64::Engine as _;
use fsc_core::Maps;
use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

pub(crate) fn unwrap_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "method": {"type": "string", "enum": ["conformal", "project"], "default": "conformal", "description": "conformal flattens each chart by least-squares conformal maps (angles kept, little stretch); project is the straight box projection (fastest, stretches faces at an angle)."},
            "seams": {"type": "string", "enum": ["direction", "groups", "charts"], "default": "direction", "description": "Where charts are cut: direction groups faces by the way they face; groups cuts along face group borders, so groups painted with the Group brush or made by group_auto place the seams (a group that is a closed shell is split further by direction); charts grows disc-shaped charts over the faces automatically (see chart_angle), merges tiny ones and cuts along their borders without touching the face groups."},
            "chart_angle": {"type": "number", "minimum": 5, "maximum": 89, "default": 45, "description": "With seams charts: a face joins a chart while its normal is within this many degrees of the chart's average normal. Smaller gives more, flatter charts."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn unwrap(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let flatten = match args.get("method").and_then(Value::as_str).unwrap_or("conformal") {
        "conformal" => fsc_paint::Flatten::Conformal,
        "project" => fsc_paint::Flatten::Project,
        other => return Err(bad(format!("unknown method {other}; conformal or project"))),
    };
    let seams = args.get("seams").and_then(Value::as_str).unwrap_or("direction");
    let auto = seams == "charts";
    let charts = match seams {
        "direction" => fsc_paint::Charts::Direction,
        "groups" => fsc_paint::Charts::Groups,
        "charts" => fsc_paint::Charts::Direction,
        other => return Err(bad(format!("unknown seams {other}; direction, groups or charts"))),
    };
    let angle = match args.get("chart_angle") {
        None | Some(Value::Null) => 45.0,
        Some(v) => v.as_f64().filter(|a| (5.0..=89.0).contains(a)).ok_or_else(|| bad("chart_angle must be 5 to 89"))?,
    };
    let id = target(s, args)?;
    let started = Instant::now();
    let object = s.scene.get_mut(id).expect("checked");
    if charts == fsc_paint::Charts::Groups && !object.mesh.has_groups() {
        return Err(CommandError::new("no_groups", "the object has no face groups; paint some with the Group brush or run group_auto, or use seams direction"));
    }
    let result = if auto {
        let seg = fsc_paint::segment(&object.mesh, angle as f32);
        fsc_paint::unwrap_keyed(&object.mesh, 4.0 / 1024.0, flatten, charts, Some(&seg.keys))
    } else {
        fsc_paint::unwrap_charts(&object.mesh, 4.0 / 1024.0, flatten, charts)
    };
    let (mean, worst) = fsc_paint::stretch(&object.mesh, &result.uvs);
    let mut mesh = object.mesh.clone();
    mesh.uvs = result.uvs;
    let mask = object.mask.clone();
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    object.clear_maps();
    let after = summary(object);
    s.history.push(Edit::Topology { label: "Unwrap UVs".into(), object: id, mesh: old_mesh, mask: old_mask });
    let round = |v: f32| (f64::from(v) * 1000.0).round() / 1000.0;
    let mut out = json!({"object": after, "charts": result.charts, "coverage": result.coverage, "stretch": {"mean": round(mean), "max": round(worst)}, "ms": started.elapsed().as_millis()});
    if auto {
        out["chart_angle"] = json!(angle);
    }
    Ok(out)
}

fn uv_point(args: &Value, key: &str) -> Result<Option<[f32; 2]>, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) if a.len() == 2 => {
            let f = |v: &Value| v.as_f64().map(|x| x as f32).ok_or_else(|| bad(format!("{key} is two numbers")));
            Ok(Some([f(&a[0])?, f(&a[1])?]))
        }
        Some(_) => Err(bad(format!("{key} is two numbers"))),
    }
}

fn need_uvs(mesh: &fsc_core::Mesh) -> Result<(), CommandError> {
    if mesh.uvs.len() != mesh.indices.len() {
        return Err(CommandError::new("no_uvs", "the object has no UVs; run uv_unwrap first (UVs are cleared by topology changes)"));
    }
    Ok(())
}

pub(crate) fn uv_layout_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "size": {"type": "integer", "minimum": 64, "maximum": 2048, "default": 512, "description": "Width and height of the picture in pixels."},
            "at": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2, "description": "A UV point [u, v] (0 to 1, v up); the island under it is drawn brighter and reported as selected."},
            "island": {"type": "integer", "minimum": 0, "description": "An island's number to draw brighter, instead of at."},
            "tile": {"type": "integer", "minimum": 1001, "maximum": 1100, "default": 1001, "description": "The UDIM tile to draw (1001 is the unit square); at is then given inside that tile, 0 to 1."},
            "overview": {"type": "boolean", "default": false, "description": "Draw every occupied UDIM tile at once instead of one tile: the smallest square of tiles holding them all, scaled to the picture. The reply's overview gives its lower left tile corner (u0, v0) and its width in tiles (n); at, island bounds and used are then in that scaled square."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// The UV layout as a picture with its islands' bounds, for looking at and for picking islands to move.
pub(crate) fn uv_layout(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let size = args.get("size").and_then(Value::as_u64).unwrap_or(512);
    if !(64..=2048).contains(&size) {
        return Err(bad("size must be 64 to 2048"));
    }
    let at = uv_point(args, "at")?;
    let tile = args.get("tile").and_then(Value::as_u64).unwrap_or(1001) as u32;
    if !(1001..=1100).contains(&tile) {
        return Err(bad("tile must be 1001 to 1100"));
    }
    let mesh = &s.scene.get(id).expect("checked").mesh;
    need_uvs(mesh)?;
    let shifted;
    let mut overview = Value::Null;
    let mesh = if args.get("overview").and_then(Value::as_bool).unwrap_or(false) {
        let tiles = crate::udim_cmds::occupied_tiles(mesh);
        let place = |t: &u32| (((t - 1001) % 10) as f32, ((t - 1001) / 10) as f32);
        let (mut u0, mut v0, mut u1, mut v1) = (f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
        for (u, v) in tiles.iter().map(place) {
            (u0, v0, u1, v1) = (u0.min(u), v0.min(v), u1.max(u), v1.max(v));
        }
        if tiles.is_empty() {
            (u0, v0, u1, v1) = (0.0, 0.0, 0.0, 0.0);
        }
        let n = (u1 - u0).max(v1 - v0) + 1.0;
        let mut copy = mesh.clone();
        for p in &mut copy.uvs {
            *p = [(p[0] - u0) / n, (p[1] - v0) / n];
        }
        overview = json!({"u0": u0, "v0": v0, "n": n, "tiles": tiles});
        shifted = copy;
        &shifted
    } else if tile == 1001 {
        mesh
    } else {
        shifted = crate::udim_cmds::shifted_to_unit(mesh, tile);
        &shifted
    };
    let island = fsc_paint::islands(mesh);
    let count = island.iter().copied().max().map_or(0, |m| m as usize + 1);
    let selected = match args.get("island").and_then(Value::as_u64) {
        Some(i) => Some(i as u32).filter(|&i| (i as usize) < count),
        None => at.and_then(|p| fsc_paint::island_at(mesh, &island, p)),
    };
    let pixels = fsc_paint::layout_image(mesh, &island, size as usize, selected);
    let png = fsc_paint::png_rgba_rect(size as usize, size as usize, &pixels).map_err(|e| CommandError::new("render_failed", e))?;
    let mut bounds = vec![[f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY]; count];
    let mut used = 0.0f64;
    for t in 0..mesh.triangle_count() {
        let q = [0, 1, 2].map(|k| mesh.uvs[3 * t + k]);
        let b = &mut bounds[island[t] as usize];
        for p in q {
            b[0] = b[0].min(p[0]);
            b[1] = b[1].min(p[1]);
            b[2] = b[2].max(p[0]);
            b[3] = b[3].max(p[1]);
        }
        used += f64::from(((q[1][0] - q[0][0]) * (q[2][1] - q[0][1]) - (q[2][0] - q[0][0]) * (q[1][1] - q[0][1])).abs()) * 0.5;
    }
    let (mean, worst) = fsc_paint::stretch(mesh, &mesh.uvs);
    let round = |v: f64| (v * 1000.0).round() / 1000.0;
    let listed: Vec<Value> = bounds.iter().take(64).enumerate().map(|(i, b)| json!({"island": i, "min": [round(f64::from(b[0])), round(f64::from(b[1]))], "max": [round(f64::from(b[2])), round(f64::from(b[3]))]})).collect();
    Ok(json!({
        "width": size, "height": size, "mime": "image/png",
        "png_base64": base64::engine::general_purpose::STANDARD.encode(&png),
        "islands": count, "listed": listed, "selected": selected, "overview": overview,
        "used": round(used), "stretch": {"mean": round(f64::from(mean)), "max": round(f64::from(worst))}
    }))
}

pub(crate) fn uv_transform_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "at": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2, "description": "A UV point [u, v] on the island to move (get_uv_layout lists island bounds)."},
            "island": {"type": "integer", "minimum": 0, "description": "The island's number from get_uv_layout, instead of at."},
            "translate": {"type": "array", "items": {"type": "number"}, "minItems": 2, "maxItems": 2, "description": "Move by [du, dv]."},
            "rotate": {"type": "number", "minimum": -360, "maximum": 360, "description": "Turn counter-clockwise by degrees about the island's centre."},
            "scale": {"type": "number", "exclusiveMinimum": 0, "maximum": 100, "description": "Scale about the island's centre."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// Moves, turns or scales one UV island (scale and turn about its box centre, then the move). One undo step.
pub(crate) fn uv_transform(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let at = uv_point(args, "at")?;
    let translate = uv_point(args, "translate")?.unwrap_or([0.0, 0.0]);
    let degrees = args.get("rotate").and_then(Value::as_f64).unwrap_or(0.0);
    let scale = args.get("scale").and_then(Value::as_f64).unwrap_or(1.0);
    if !(scale > 0.0 && scale <= 100.0) || !(-360.0..=360.0).contains(&degrees) {
        return Err(bad("scale is above 0 up to 100 and rotate -360 to 360"));
    }
    let object = s.scene.get_mut(id).expect("checked");
    need_uvs(&object.mesh)?;
    let island = fsc_paint::islands(&object.mesh);
    let chosen = match (args.get("island").and_then(Value::as_u64), at) {
        (Some(i), _) => i as u32,
        (None, Some(p)) => fsc_paint::island_at(&object.mesh, &island, p).ok_or_else(|| CommandError::new("no_island", "no UV island under that point; get_uv_layout shows where they are"))?,
        (None, None) => return Err(bad("give at or island")),
    };
    let corners: Vec<usize> = (0..object.mesh.indices.len()).filter(|&c| island[c / 3] == chosen).collect();
    if corners.is_empty() {
        return Err(CommandError::new("no_island", format!("no island {chosen}; get_uv_layout counts them")));
    }
    let (mut lo, mut hi) = ([f32::INFINITY; 2], [f32::NEG_INFINITY; 2]);
    for &c in &corners {
        let p = object.mesh.uvs[c];
        lo = [lo[0].min(p[0]), lo[1].min(p[1])];
        hi = [hi[0].max(p[0]), hi[1].max(p[1])];
    }
    let centre = [(lo[0] + hi[0]) * 0.5, (lo[1] + hi[1]) * 0.5];
    let (sin, cos) = (degrees.to_radians() as f32).sin_cos();
    let k = scale as f32;
    let mut mesh = object.mesh.clone();
    for &c in &corners {
        let p = mesh.uvs[c];
        let (x, y) = ((p[0] - centre[0]) * k, (p[1] - centre[1]) * k);
        mesh.uvs[c] = [centre[0] + x * cos - y * sin + translate[0], centre[1] + x * sin + y * cos + translate[1]];
    }
    let mask = object.mask.clone();
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    object.clear_maps();
    let after = summary(object);
    s.history.push(Edit::Topology { label: "Move UV island".into(), object: id, mesh: old_mesh, mask: old_mask });
    Ok(json!({"object": after, "island": chosen, "corners": corners.len()}))
}

/// Packs the UV islands back into the unit square, keeping their shapes and turns and their sizes relative to each
/// other (after islands were moved or scaled by hand). One undo step.
pub(crate) fn packed_mesh(source: &fsc_core::Mesh) -> (fsc_core::Mesh, usize, f32) {
    let island = fsc_paint::islands(source);
    let count = island.iter().copied().max().map_or(0, |m| m as usize + 1);
    let mut charts: Vec<Vec<usize>> = vec![Vec::new(); count];
    for c in 0..source.indices.len() {
        charts[island[c / 3] as usize].push(c);
    }
    let mut coords: Vec<glam::Vec2> = source.uvs.iter().map(|&p| glam::Vec2::from_array(p)).collect();
    let coverage = fsc_paint::pack(&charts, &mut coords, 4.0 / 1024.0);
    let mut mesh = source.clone();
    mesh.uvs = coords.iter().map(|q| q.to_array()).collect();
    (mesh, count, coverage)
}

pub(crate) fn uv_pack(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get_mut(id).expect("checked");
    need_uvs(&object.mesh)?;
    let (mesh, count, coverage) = packed_mesh(&object.mesh);
    let mask = object.mask.clone();
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    object.clear_maps();
    let after = summary(object);
    s.history.push(Edit::Topology { label: "Pack UVs".into(), object: id, mesh: old_mesh, mask: old_mask });
    Ok(json!({"object": after, "islands": count, "coverage": coverage}))
}

pub(crate) fn bake_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "The object to bake onto (it needs UVs from uv_unwrap); the active object when omitted."},
            "size": {"type": "integer", "enum": [256, 512, 1024, 2048, 4096], "default": 1024, "description": "Texture width and height in pixels."},
            "source_id": {"type": "integer", "minimum": 1, "description": "Bake detail from this object (a high-resolution sculpt) instead; by default the object's highest subdivision level, or the object itself."},
            "path": {"type": "string", "minLength": 1, "description": "A folder to also write NAME_color.png, NAME_roughness_metal.png, NAME_normal.png and NAME_displacement.exr into (NAME_color.1001.png and so on when tile or udim is set). Optional, also with udim."},
            "tile": {"type": "integer", "minimum": 1001, "maximum": 1100, "description": "Bake only this UDIM tile: the UVs are shifted by minus the tile's offset so the tile fills the texture, and texels outside the tile are left uncovered."},
            "udim": {"type": "boolean", "default": false, "description": "Bake every UDIM tile that holds UV islands (as uv_tiles lists them), one set of maps per tile, written as NAME_color.1001.png, NAME_roughness_metal.1001.png, NAME_normal.1001.png, NAME_displacement.1001.exr (and NAME_emissive.1001.png when it glows) when path is given. Every baked tile is also kept on the object, so GLB and glTF export gives one primitive and one material per tile with that tile's maps. Not together with tile."}
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
    let tile = match args.get("tile") {
        None | Some(Value::Null) => None,
        Some(v) => Some(v.as_u64().filter(|n| (1001..=1100).contains(n)).ok_or_else(|| bad("tile is a tile number, 1001 to 1100"))? as u32),
    };
    let udim = match args.get("udim") {
        None | Some(Value::Null) => false,
        Some(v) => v.as_bool().ok_or_else(|| bad("udim is true or false"))?,
    };
    if udim && tile.is_some() {
        return Err(bad("give tile or udim, not both"));
    }
    if (udim || tile.is_some()) && object.mesh.uvs.len() != object.mesh.indices.len() {
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
    let jobs: Vec<Option<u32>> = if udim {
        let tiles = crate::udim_cmds::occupied_tiles(&object.mesh);
        if tiles.is_empty() {
            return Err(CommandError::new("no_tiles", "no UV island has its centre inside a UDIM tile (u 0 to 10, v 0 and up); see uv_tiles"));
        }
        tiles.into_iter().map(Some).collect()
    } else {
        vec![tile]
    };
    let dir = args.get("path").and_then(Value::as_str);
    let stem: String = object.name.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    let png = |px: &[[u8; 4]]| fsc_paint::png_rgba(size, px).map_err(|e| CommandError::new("encode_failed", e));
    let texels = size * size;
    let mut kept: Option<(Maps, f32, f32, [f32; 2])> = None;
    let mut written = Vec::new();
    let mut per_tile: Vec<(u32, Maps)> = Vec::new();
    for job in &jobs {
        let shifted;
        let mesh = match job {
            Some(t) if *t != 1001 => {
                shifted = crate::udim_cmds::shifted_to_unit(&object.mesh, *t);
                &shifted
            }
            _ => &object.mesh,
        };
        let ex = fsc_paint::expand(mesh);
        let baked = fsc_paint::bake(mesh, &ex, source_mesh, source_bvh, size, max_dist, 6);
        let maps = Maps {
            size,
            color_png: png(&baked.color)?,
            mr_png: png(&baked.mr)?,
            normal_png: png(&baked.normal)?,
            displacement_exr: fsc_paint::exr_float(size, &baked.displacement),
            emissive_png: if baked.glows { png(&baked.emissive)? } else { Vec::new() },
            source: source_name.clone(),
        };
        let (lo, hi) = baked.displacement.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), &d| (a.min(d), b.max(d)));
        if let Some(dir) = dir {
            for (name, ext, bytes) in [("color", "png", &maps.color_png), ("roughness_metal", "png", &maps.mr_png), ("normal", "png", &maps.normal_png), ("displacement", "exr", &maps.displacement_exr), ("emissive", "png", &maps.emissive_png)] {
                if bytes.is_empty() {
                    continue;
                }
                let leaf = match job {
                    Some(t) => format!("{stem}_{name}.{t}.{ext}"),
                    None => format!("{stem}_{name}.{ext}"),
                };
                let file = Path::new(dir).join(leaf);
                std::fs::write(&file, bytes).map_err(|e| CommandError::new("io_failed", format!("could not write {}: {e}", file.display())))?;
                written.push(file.display().to_string());
            }
        }
        let covered = baked.covered as f32 / texels as f32;
        let hit = if baked.covered > 0 { baked.hits as f32 / baked.covered as f32 } else { 0.0 };
        if let (true, Some(t)) = (udim, job) {
            per_tile.push((*t, maps.clone()));
        }
        if kept.is_none() || *job == Some(1001) {
            kept = Some((maps, covered, hit, [lo, hi]));
        }
    }
    let (maps, covered, hit, range) = kept.expect("at least one tile");
    let object = s.scene.get_mut(id).expect("checked");
    object.maps = Some(maps);
    object.tile_maps = per_tile;
    object.revision += 1;
    let mut out = json!({
        "id": id,
        "size": size,
        "source": source_name,
        "covered": covered,
        "hit": hit,
        "displacement_range": range,
        "files": written,
        "ms": started.elapsed().as_millis(),
    });
    if udim || tile.is_some() {
        out["tiles"] = json!(jobs.iter().flatten().collect::<Vec<_>>());
    }
    Ok(out)
}
