use std::collections::BTreeMap;

use fsc_core::Mesh;
use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

const EPS: f32 = 1e-5;

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn need_uvs(mesh: &Mesh) -> Result<(), CommandError> {
    if mesh.uvs.len() != mesh.indices.len() {
        return Err(CommandError::new("no_uvs", "the object has no UVs; run uv_unwrap first (UVs are cleared by topology changes)"));
    }
    Ok(())
}

fn round(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

struct Layout {
    island: Vec<u32>,
    lo: Vec<[f32; 2]>,
    hi: Vec<[f32; 2]>,
    area: Vec<f64>,
    faces: Vec<usize>,
}

impl Layout {
    fn count(&self) -> usize {
        self.area.len()
    }

    fn centre(&self, i: usize) -> [f32; 2] {
        [(self.lo[i][0] + self.hi[i][0]) * 0.5, (self.lo[i][1] + self.hi[i][1]) * 0.5]
    }

    fn cell(&self, i: usize) -> Option<(u32, u32)> {
        let c = self.centre(i);
        (c[0] >= 0.0 && c[0] < 10.0 && c[1] >= 0.0).then(|| (c[0].floor() as u32, c[1].floor() as u32))
    }

    fn crosses(&self, i: usize) -> bool {
        (0..2).any(|k| (self.hi[i][k] - EPS).floor() > (self.lo[i][k] + EPS).floor())
    }
}

fn measure(mesh: &Mesh) -> Layout {
    let island = fsc_paint::islands(mesh);
    let count = island.iter().copied().max().map_or(0, |m| m as usize + 1);
    let mut l = Layout { island, lo: vec![[f32::INFINITY; 2]; count], hi: vec![[f32::NEG_INFINITY; 2]; count], area: vec![0.0; count], faces: vec![0; count] };
    for t in 0..mesh.triangle_count() {
        let q = [0, 1, 2].map(|k| mesh.uvs[3 * t + k]);
        let i = l.island[t] as usize;
        for p in q {
            l.lo[i] = [l.lo[i][0].min(p[0]), l.lo[i][1].min(p[1])];
            l.hi[i] = [l.hi[i][0].max(p[0]), l.hi[i][1].max(p[1])];
        }
        l.area[i] += f64::from(((q[1][0] - q[0][0]) * (q[2][1] - q[0][1]) - (q[2][0] - q[0][0]) * (q[1][1] - q[0][1])).abs()) * 0.5;
        l.faces[i] += 1;
    }
    l
}

fn tile_number(u: u32, v: u32) -> u32 {
    1001 + u + 10 * v
}

fn tile_arg(args: &Value, key: &str) -> Result<Option<u32>, CommandError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => match v.as_u64() {
            Some(n) if (1001..=1100).contains(&n) => Ok(Some(n as u32)),
            _ => Err(bad(format!("{key} is a tile number, 1001 to 1100"))),
        },
    }
}

fn by_tile(l: &Layout) -> BTreeMap<u32, Vec<usize>> {
    let mut out: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for i in 0..l.count() {
        if let Some((u, v)) = l.cell(i) {
            out.entry(tile_number(u, v)).or_default().push(i);
        }
    }
    out
}

/// The tile numbers that hold at least one UV island centre, ascending.
pub(crate) fn occupied_tiles(mesh: &Mesh) -> Vec<u32> {
    by_tile(&measure(mesh)).keys().copied().collect()
}

/// The UDIM tile of each triangle by the island's box centre (0 where the island is outside every tile).
pub(crate) fn triangle_tiles(mesh: &Mesh) -> Vec<u32> {
    let l = measure(mesh);
    let tile: Vec<u32> = (0..l.count()).map(|i| l.cell(i).map_or(0, |(u, v)| tile_number(u, v))).collect();
    l.island.iter().map(|&i| tile[i as usize]).collect()
}

/// The export split for an object with per-tile maps and UVs on every corner; None otherwise.
pub(crate) fn tile_split(o: &fsc_core::Object) -> Option<fsc_io::TileSplit<'_>> {
    (!o.tile_maps.is_empty() && !o.mesh.uvs.is_empty() && o.mesh.uvs.len() == o.mesh.indices.len()).then(|| fsc_io::TileSplit { of_triangle: triangle_tiles(&o.mesh), maps: o.tile_maps.iter().map(|(t, m)| (*t, m)).collect() })
}

/// A copy of the mesh with its UVs moved by minus the tile's (u, v) offset, so the tile becomes the unit square.
pub(crate) fn shifted_to_unit(mesh: &Mesh, tile: u32) -> Mesh {
    let (du, dv) = (((tile - 1001) % 10) as f32, ((tile - 1001) / 10) as f32);
    let mut copy = mesh.clone();
    for p in &mut copy.uvs {
        *p = [p[0] - du, p[1] - dv];
    }
    copy
}

pub(crate) fn uv_tiles_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// Lists the UDIM tiles the UV islands sit in, with the islands, faces and area coverage of each.
pub(crate) fn uv_tiles(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let mesh = &s.scene.get(id).expect("checked").mesh;
    need_uvs(mesh)?;
    let l = measure(mesh);
    let tiles: Vec<Value> = by_tile(&l)
        .iter()
        .map(|(&tile, members)| {
            let faces: usize = members.iter().map(|&i| l.faces[i]).sum();
            let area: f64 = members.iter().map(|&i| l.area[i]).sum();
            json!({"tile": tile, "u": (tile - 1001) % 10, "v": (tile - 1001) / 10, "islands": members, "faces": faces, "coverage": round(area.min(1.0))})
        })
        .collect();
    let crossing: Vec<usize> = (0..l.count()).filter(|&i| l.crosses(i)).collect();
    let outside: Vec<usize> = (0..l.count()).filter(|&i| l.cell(i).is_none()).collect();
    Ok(json!({"tiles": tiles, "crossing": crossing, "outside": outside}))
}

pub(crate) fn uv_to_tile_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "tile": {"type": "integer", "minimum": 1001, "maximum": 1100, "description": "The UDIM tile to move the islands into (1001 is u 0 to 1, v 0 to 1; 1002 is the next tile along u; 1011 starts the next row)."},
            "islands": {"type": "array", "items": {"type": "integer", "minimum": 0}, "description": "Island numbers from get_uv_layout or uv_tiles; every island when omitted."}
        },
        "required": ["tile"],
        "additionalProperties": false
    })
}

/// Moves UV islands by whole tiles so their centres land in one UDIM tile, keeping their place inside the tile. One undo step.
pub(crate) fn uv_to_tile(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let tile = tile_arg(args, "tile")?.ok_or_else(|| bad("tile is required, 1001 to 1100"))?;
    let object = s.scene.get_mut(id).expect("checked");
    need_uvs(&object.mesh)?;
    let l = measure(&object.mesh);
    let mut chosen = vec![args.get("islands").is_none_or(Value::is_null); l.count()];
    if let Some(list) = args.get("islands").filter(|v| !v.is_null()) {
        let list = list.as_array().ok_or_else(|| bad("islands is a list of island numbers"))?;
        for v in list {
            let i = v.as_u64().filter(|&i| (i as usize) < l.count()).ok_or_else(|| bad(format!("islands holds a number from 0 to {}", l.count().saturating_sub(1))))?;
            chosen[i as usize] = true;
        }
    }
    let (tu, tv) = (((tile - 1001) % 10) as i64, ((tile - 1001) / 10) as i64);
    let offsets: Vec<[f32; 2]> = (0..l.count())
        .map(|i| {
            let c = l.centre(i);
            [(tu - i64::from(c[0].floor() as i32)) as f32, (tv - i64::from(c[1].floor() as i32)) as f32]
        })
        .collect();
    let mut mesh = object.mesh.clone();
    let mut moved = 0;
    for i in (0..l.count()).filter(|&i| chosen[i]) {
        if offsets[i] != [0.0, 0.0] {
            moved += 1;
        }
    }
    for (c, p) in mesh.uvs.iter_mut().enumerate() {
        let i = l.island[c / 3] as usize;
        if chosen[i] {
            *p = [p[0] + offsets[i][0], p[1] + offsets[i][1]];
        }
    }
    let mask = object.mask.clone();
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    let texture = object.texture.take();
    object.clear_maps();
    let after = summary(object);
    s.history.push(Edit::Topology { label: "Move UVs to Tile".into(), object: id, mesh: old_mesh, mask: old_mask }.with_texture(id, texture));
    Ok(json!({"object": after, "moved": moved, "islands": chosen.iter().filter(|&&c| c).count(), "tile": tile}))
}

pub(crate) fn uv_pack_udim_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "by": {"type": "string", "enum": ["group", "island"], "default": "group", "description": "group gives each face group its own tile (an island goes to the group most of its faces belong to; tiles follow ascending group number); island spreads the islands over tiles, balancing UV area."},
            "tiles": {"type": "integer", "minimum": 0, "maximum": 100, "default": 0, "description": "With by island: how many tiles to use; 0 picks enough that each holds about the UV area of the largest island."},
            "start": {"type": "integer", "minimum": 1001, "maximum": 1100, "default": 1001, "description": "The first tile number; tiles go along u and wrap to the next row after ten."}
        },
        "required": [],
        "additionalProperties": false
    })
}

/// Sorts UV islands into UDIM tiles by face group or by balanced area and packs each tile's islands into its tile. One undo step.
pub(crate) fn uv_pack_udim(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let by = args.get("by").and_then(Value::as_str).unwrap_or("group");
    if by != "group" && by != "island" {
        return Err(bad(format!("unknown by {by}; group or island")));
    }
    let start = tile_arg(args, "start")?.unwrap_or(1001);
    let wanted = match args.get("tiles") {
        None | Some(Value::Null) => 0,
        Some(v) => v.as_u64().filter(|&n| n <= 100).ok_or_else(|| bad("tiles is 0 to 100"))? as usize,
    };
    let object = s.scene.get_mut(id).expect("checked");
    need_uvs(&object.mesh)?;
    let l = measure(&object.mesh);
    if l.count() == 0 {
        return Err(CommandError::new("no_uvs", "the object has no triangles to pack"));
    }
    let mut bins: Vec<Vec<usize>> = Vec::new();
    if by == "group" {
        if !object.mesh.has_groups() {
            return Err(CommandError::new("no_groups", "the object has no face groups; make some with group_auto or group_from_mask, or use by island"));
        }
        let mut votes: Vec<BTreeMap<u32, usize>> = vec![BTreeMap::new(); l.count()];
        for (t, &i) in l.island.iter().enumerate() {
            *votes[i as usize].entry(object.mesh.groups[t]).or_default() += 1;
        }
        let mut owned: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
        for (i, v) in votes.iter().enumerate() {
            let (mut best, mut most) = (0, 0);
            for (&g, &n) in v {
                if n > most {
                    best = g;
                    most = n;
                }
            }
            owned.entry(best).or_default().push(i);
        }
        bins = owned.into_values().collect();
    } else {
        let total: f64 = l.area.iter().sum();
        let largest = l.area.iter().copied().fold(0.0, f64::max);
        let n = if wanted > 0 {
            wanted
        } else if largest > 0.0 {
            ((total / largest).ceil() as usize).max(1)
        } else {
            1
        };
        let n = n.min(l.count()).min(100);
        let mut order: Vec<usize> = (0..l.count()).collect();
        order.sort_by(|&a, &b| l.area[b].total_cmp(&l.area[a]).then(a.cmp(&b)));
        let mut load = vec![0.0f64; n];
        bins.resize(n, Vec::new());
        for i in order {
            let k = (0..n).fold(0, |best, k| if load[k] < load[best] { k } else { best });
            load[k] += l.area[i];
            bins[k].push(i);
        }
        bins.retain(|b| !b.is_empty());
    }
    let (su, sv) = ((start - 1001) % 10, (start - 1001) / 10);
    if sv + (su + bins.len() as u32 - 1) / 10 > 9 {
        return Err(bad(format!("{} tiles starting at {start} do not fit below tile 1100", bins.len())));
    }
    let mut corners: Vec<Vec<usize>> = vec![Vec::new(); l.count()];
    for c in 0..object.mesh.indices.len() {
        corners[l.island[c / 3] as usize].push(c);
    }
    let mut coords: Vec<glam::Vec2> = object.mesh.uvs.iter().map(|&p| glam::Vec2::from_array(p)).collect();
    let mut listed = Vec::new();
    for (k, bin) in bins.iter().enumerate() {
        let at = su + k as u32;
        let (u, v) = (at % 10, sv + at / 10);
        let charts: Vec<Vec<usize>> = bin.iter().map(|&i| corners[i].clone()).collect();
        let coverage = fsc_paint::pack(&charts, &mut coords, 4.0 / 1024.0);
        for &c in charts.iter().flatten() {
            coords[c] += glam::Vec2::new(u as f32, v as f32);
        }
        listed.push(json!({"tile": tile_number(u, v), "islands": bin.len(), "coverage": round(f64::from(coverage))}));
    }
    let mut mesh = object.mesh.clone();
    mesh.uvs = coords.iter().map(|q| q.to_array()).collect();
    let mask = object.mask.clone();
    let (old_mesh, old_mask) = object.replace_mesh(mesh, mask);
    let texture = object.texture.take();
    object.clear_maps();
    s.history.push(Edit::Topology { label: "Pack UVs into Tiles".into(), object: id, mesh: old_mesh, mask: old_mask }.with_texture(id, texture));
    Ok(json!({"tiles": listed, "islands": l.count()}))
}
