use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::{json, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

const SPLIT: f32 = 1e-4;
const MIN_SPLIT: f32 = 2e-6;

type EdgeSide = (usize, usize, usize);

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn same(a: [f32; 2], b: [f32; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6
}

fn island_count(island: &[u32]) -> usize {
    island.iter().copied().max().map_or(0, |m| m as usize + 1)
}

fn need_uvs(mesh: &fsc_core::Mesh) -> Result<(), CommandError> {
    if mesh.uvs.len() != mesh.indices.len() || mesh.indices.is_empty() {
        return Err(CommandError::new("no_uvs", "the object has no UVs; run uv_unwrap first (UVs are cleared by topology changes)"));
    }
    Ok(())
}

fn uv_area(mesh: &fsc_core::Mesh, t: usize) -> f64 {
    let (a, b, c) = (mesh.uvs[3 * t], mesh.uvs[3 * t + 1], mesh.uvs[3 * t + 2]);
    0.5 * (f64::from(b[0] - a[0]) * f64::from(c[1] - a[1]) - f64::from(c[0] - a[0]) * f64::from(b[1] - a[1]))
}

pub(crate) fn uv_sew_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "island_a": {"type": "integer", "minimum": 0, "description": "The island that stays where it is, by its number from get_uv_layout."},
            "island_b": {"type": "integer", "minimum": 0, "description": "The island that is turned, scaled and moved onto island_a, by its number from get_uv_layout."}
        },
        "required": ["island_a", "island_b"],
        "additionalProperties": false
    })
}

fn island_arg(args: &Value, key: &str, count: usize) -> Result<u32, CommandError> {
    match args.get(key).and_then(Value::as_u64) {
        Some(i) if (i as usize) < count => Ok(i as u32),
        Some(_) => Err(bad(format!("{key} is an island number from 0 to {}; get_uv_layout counts them", count.saturating_sub(1)))),
        None => Err(bad(format!("{key} is an island number from get_uv_layout"))),
    }
}

/// Sews island_b onto island_a along the mesh edges they share. One undo step.
pub(crate) fn uv_sew(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let object = s.scene.get_mut(id).expect("checked");
    need_uvs(&object.mesh)?;
    let island = fsc_paint::islands(&object.mesh);
    let before = island_count(&island);
    let a = island_arg(args, "island_a", before)?;
    let b = island_arg(args, "island_b", before)?;
    if a == b {
        return Err(bad("island_a and island_b are the same island"));
    }
    let mesh = &object.mesh;
    let mut edges: HashMap<(u32, u32), Vec<EdgeSide>> = HashMap::new();
    for t in 0..mesh.triangle_count() {
        for k in 0..3 {
            let (i, j) = (3 * t + k, 3 * t + (k + 1) % 3);
            let (u, v) = (mesh.indices[i], mesh.indices[j]);
            let entry = if u < v { (t, i, j) } else { (t, j, i) };
            edges.entry((u.min(v), u.max(v))).or_default().push(entry);
        }
    }
    let mut keys: Vec<&(u32, u32)> = edges.keys().collect();
    keys.sort_unstable();
    let mut shared = 0usize;
    let mut pairs: Vec<([f64; 2], [f64; 2])> = Vec::new();
    let mut weld: HashMap<(u32, u32, u32), [f32; 2]> = HashMap::new();
    for key in keys {
        let list = &edges[key];
        let mut found = false;
        for &(ta, a_lo, a_hi) in list.iter().filter(|e| island[e.0] == a) {
            for &(tb, b_lo, b_hi) in list.iter().filter(|e| island[e.0] == b) {
                let (sa, sb) = (uv_area(mesh, ta), uv_area(mesh, tb));
                if sa * sb < 0.0 {
                    return Err(CommandError::new("needs_mirror", "the two islands face opposite ways in UV space, so sewing them would need a mirror; flip one island first"));
                }
                found = true;
                for (ca, cb, v) in [(a_lo, b_lo, key.0), (a_hi, b_hi, key.1)] {
                    let (qa, pb) = (mesh.uvs[ca], mesh.uvs[cb]);
                    pairs.push(([f64::from(pb[0]), f64::from(pb[1])], [f64::from(qa[0]), f64::from(qa[1])]));
                    weld.entry((v, pb[0].to_bits(), pb[1].to_bits())).or_insert(qa);
                }
            }
        }
        if found {
            shared += 1;
        }
    }
    if shared == 0 {
        return Err(CommandError::new("no_shared_edges", format!("islands {a} and {b} share no mesh edge, so they cannot be sewn")));
    }
    let n = pairs.len() as f64;
    let (mut mp, mut mq) = ([0.0f64; 2], [0.0f64; 2]);
    for (p, q) in &pairs {
        mp = [mp[0] + p[0] / n, mp[1] + p[1] / n];
        mq = [mq[0] + q[0] / n, mq[1] + q[1] / n];
    }
    let (mut dot, mut cross, mut norm) = (0.0f64, 0.0f64, 0.0f64);
    for (p, q) in &pairs {
        let (px, py, qx, qy) = (p[0] - mp[0], p[1] - mp[1], q[0] - mq[0], q[1] - mq[1]);
        dot += px * qx + py * qy;
        cross += px * qy - py * qx;
        norm += px * px + py * py;
    }
    if norm < 1e-18 || (dot * dot + cross * cross).sqrt() < 1e-18 {
        return Err(CommandError::new("degenerate_edges", "the shared edges have no length in UV space, so no fit can be made"));
    }
    let (c, sn) = (dot / norm, cross / norm);

    let mut edited = mesh.clone();
    let mut welded = 0usize;
    for corner in 0..edited.indices.len() {
        if island[corner / 3] != b {
            continue;
        }
        let p = mesh.uvs[corner];
        if let Some(&q) = weld.get(&(mesh.indices[corner], p[0].to_bits(), p[1].to_bits())) {
            edited.uvs[corner] = q;
            welded += 1;
            continue;
        }
        let (x, y) = (f64::from(p[0]) - mp[0], f64::from(p[1]) - mp[1]);
        edited.uvs[corner] = [(mq[0] + c * x - sn * y) as f32, (mq[1] + sn * x + c * y) as f32];
    }
    let after = island_count(&fsc_paint::islands(&edited));
    let mask = object.mask.clone();
    let (old_mesh, old_mask) = object.replace_mesh(edited, mask);
    let texture = object.texture.take();
    object.clear_maps();
    let result = summary(object);
    s.history.push(Edit::Topology { label: "Sew UV islands".into(), object: id, mesh: old_mesh, mask: old_mask }.with_texture(id, texture));
    Ok(json!({"object": result, "welded": welded, "shared_edges": shared, "islands_before": before, "islands_after": after}))
}

pub(crate) fn uv_cut_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": "Object id; the active object when omitted."},
            "path": {"type": "array", "items": {"type": "integer", "minimum": 0}, "minItems": 2, "description": "Mesh vertex indices along the cut; each consecutive pair must be a mesh edge. Repeat the first vertex at the end for a closed loop. The side to the right of the walking direction (seen from outside) is the one that comes loose."},
            "separate": {"type": "boolean", "default": true, "description": "Move each piece that comes loose away from the rest of its island so the cut is visible; false leaves the pieces in place with a hairline gap."},
            "distance": {"type": "number", "minimum": 0.0005, "maximum": 0.5, "default": 0.02, "description": "How far a loose piece moves, in UV units, when separate is true."}
        },
        "required": ["path"],
        "additionalProperties": false
    })
}

fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

/// Cuts a seam along a path of mesh edges by splitting the UVs of the corners on its right side. One undo step.
pub(crate) fn uv_cut(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let separate = match args.get("separate") {
        None | Some(Value::Null) => true,
        Some(v) => v.as_bool().ok_or_else(|| bad("separate is true or false"))?,
    };
    let distance = match args.get("distance") {
        None | Some(Value::Null) => 0.02,
        Some(v) => match v.as_f64() {
            Some(x) if (0.0005..=0.5).contains(&x) => x as f32,
            _ => return Err(bad("distance is a number from 0.0005 to 0.5")),
        },
    };
    let object = s.scene.get_mut(id).expect("checked");
    need_uvs(&object.mesh)?;
    let verts = object.mesh.positions.len();
    let path: Vec<u32> = match args.get("path") {
        Some(Value::Array(list)) if list.len() >= 2 => {
            let mut out = Vec::with_capacity(list.len());
            for v in list {
                match v.as_u64() {
                    Some(i) if (i as usize) < verts => out.push(i as u32),
                    _ => return Err(bad(format!("path holds vertex indices from 0 to {}", verts.saturating_sub(1)))),
                }
            }
            out
        }
        _ => return Err(bad("path is an array of two or more vertex indices")),
    };
    let mesh = &object.mesh;
    let before_island = fsc_paint::islands(mesh);
    let before = island_count(&before_island);
    let mut edge_tris: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for t in 0..mesh.triangle_count() {
        for k in 0..3 {
            let (u, v) = (mesh.indices[3 * t + k], mesh.indices[3 * t + (k + 1) % 3]);
            edge_tris.entry((u.min(v), u.max(v))).or_default().push(t);
        }
    }
    let mut cut: HashSet<(u32, u32)> = HashSet::new();
    let mut left_at: HashSet<(u32, usize)> = HashSet::new();
    let mut right_at: HashSet<(u32, usize)> = HashSet::new();
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let key = (a.min(b), a.max(b));
        let Some(list) = edge_tris.get(&key) else {
            return Err(bad(format!("vertices {a} and {b} are not joined by a mesh edge")));
        };
        cut.insert(key);
        for &t in list {
            for k in 0..3 {
                let (x, y) = (mesh.indices[3 * t + k], mesh.indices[3 * t + (k + 1) % 3]);
                if (x, y) == (a, b) {
                    left_at.insert((a, t));
                    left_at.insert((b, t));
                } else if (x, y) == (b, a) {
                    right_at.insert((a, t));
                    right_at.insert((b, t));
                }
            }
        }
    }
    let on_path: HashSet<u32> = path.iter().copied().collect();
    let mut around: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();
    for (c, &v) in mesh.indices.iter().enumerate() {
        if on_path.contains(&v) {
            around.entry(v).or_default().push((c / 3, c));
        }
    }
    let mut edited = mesh.clone();
    let mut changed = 0usize;
    for (&v, list) in &around {
        let at: HashMap<usize, usize> = list.iter().enumerate().map(|(i, &(t, _))| (t, i)).collect();
        let mut parent: Vec<usize> = (0..list.len()).collect();
        for (i, &(t, c)) in list.iter().enumerate() {
            for w in [mesh.indices[3 * t + (c % 3 + 1) % 3], mesh.indices[3 * t + (c % 3 + 2) % 3]] {
                let key = (v.min(w), v.max(w));
                if cut.contains(&key) {
                    continue;
                }
                for &u in &edge_tris[&key] {
                    if let Some(&j) = at.get(&u) {
                        if u != t && same(mesh.uvs[c], mesh.uvs[list[j].1]) {
                            let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                            parent[ri] = rj;
                        }
                    }
                }
            }
        }
        let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for i in 0..list.len() {
            let r = find(&mut parent, i);
            groups.entry(r).or_default().push(i);
        }
        for members in groups.values() {
            let left = members.iter().any(|&i| left_at.contains(&(v, list[i].0)));
            let right = members.iter().any(|&i| right_at.contains(&(v, list[i].0)));
            if !right || left {
                continue;
            }
            let origin = mesh.uvs[list[members[0]].1];
            let (mut sum, mut near, mut count) = ([0.0f32; 2], f32::INFINITY, 0.0f32);
            for &i in members {
                let (t, c) = list[i];
                for other in [3 * t + (c % 3 + 1) % 3, 3 * t + (c % 3 + 2) % 3] {
                    let p = mesh.uvs[other];
                    sum = [sum[0] + p[0], sum[1] + p[1]];
                    count += 1.0;
                    near = near.min(((p[0] - origin[0]).powi(2) + (p[1] - origin[1]).powi(2)).sqrt());
                }
            }
            let to = [sum[0] / count - origin[0], sum[1] / count - origin[1]];
            let len = (to[0] * to[0] + to[1] * to[1]).sqrt();
            let mag = SPLIT.min(0.25 * near);
            if len < 1e-12 || mag < MIN_SPLIT {
                continue;
            }
            for &i in members {
                let c = list[i].1;
                edited.uvs[c] = [edited.uvs[c][0] + to[0] / len * mag, edited.uvs[c][1] + to[1] / len * mag];
                changed += 1;
            }
        }
    }
    if changed == 0 {
        return Err(CommandError::new("no_seam_created", "the path does not separate any corners: a cut needs an interior vertex (three or more path vertices) or a path that reaches a mesh border"));
    }
    let mut island = fsc_paint::islands(&edited);
    if separate {
        let mut pieces: BTreeMap<u32, BTreeMap<u32, usize>> = BTreeMap::new();
        for t in 0..island.len() {
            *pieces.entry(before_island[t]).or_default().entry(island[t]).or_insert(0) += 1;
        }
        let centre = |piece: u32, uvs: &[[f32; 2]]| {
            let (mut sum, mut n) = ([0.0f32; 2], 0.0f32);
            for c in (0..uvs.len()).filter(|&c| island[c / 3] == piece) {
                sum = [sum[0] + uvs[c][0], sum[1] + uvs[c][1]];
                n += 1.0;
            }
            [sum[0] / n, sum[1] / n]
        };
        let mut moves: Vec<(u32, [f32; 2])> = Vec::new();
        for parts in pieces.values().filter(|p| p.len() > 1) {
            let main = parts.iter().max_by_key(|(&k, &n)| (n, std::cmp::Reverse(k))).map(|(&k, _)| k).expect("not empty");
            let home = centre(main, &edited.uvs);
            for &piece in parts.keys().filter(|&&k| k != main) {
                let at = centre(piece, &edited.uvs);
                let (dx, dy) = (at[0] - home[0], at[1] - home[1]);
                let len = (dx * dx + dy * dy).sqrt();
                let dir = if len < 1e-9 { [1.0, 0.0] } else { [dx / len, dy / len] };
                moves.push((piece, [dir[0] * distance, dir[1] * distance]));
            }
        }
        for (piece, by) in moves {
            for c in (0..edited.uvs.len()).filter(|&c| island[c / 3] == piece) {
                edited.uvs[c] = [edited.uvs[c][0] + by[0], edited.uvs[c][1] + by[1]];
            }
        }
        island = fsc_paint::islands(&edited);
    }
    let after = island_count(&island);
    let mask = object.mask.clone();
    let (old_mesh, old_mask) = object.replace_mesh(edited, mask);
    let texture = object.texture.take();
    object.clear_maps();
    let result = summary(object);
    s.history.push(Edit::Topology { label: "Cut UV seam".into(), object: id, mesh: old_mesh, mask: old_mask }.with_texture(id, texture));
    Ok(json!({"object": result, "cut_edges": cut.len(), "islands_before": before, "islands_after": after}))
}
