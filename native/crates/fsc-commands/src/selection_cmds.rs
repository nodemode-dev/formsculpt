use fsc_core::{ChannelState, Object, ObjectId, Paint, PaintLayers, SelectionSet, SET_CHANNELS};
use serde_json::{json, Map, Value};

use crate::history::Edit;
use crate::scene_cmds::summary;
use crate::sculpt_cmds::target;
use crate::{CommandError, Session};

const ID: &str = "Object id; the active object when omitted.";
const LIMIT: usize = 64;

#[derive(Debug, Clone)]
pub(crate) enum Clip {
    Groups(Vec<u32>),
    Mask(Vec<f32>),
    Paint(Vec<Paint>),
}

impl Clip {
    fn channel(&self) -> &'static str {
        match self {
            Clip::Groups(_) => "groups",
            Clip::Mask(_) => "mask",
            Clip::Paint(_) => "paint",
        }
    }

    fn len(&self) -> usize {
        match self {
            Clip::Groups(g) => g.len(),
            Clip::Mask(m) => m.len(),
            Clip::Paint(p) => p.len(),
        }
    }
}

fn bad(msg: impl Into<String>) -> CommandError {
    CommandError::new("bad_arguments", msg)
}

fn set_name(args: &Value) -> Result<String, CommandError> {
    args.get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|n| !n.is_empty() && n.chars().count() <= 64)
        .map(str::to_string)
        .ok_or_else(|| bad("name is a string of 1 to 64 characters"))
}

fn channel_list(args: &Value) -> Result<Option<Vec<&'static str>>, CommandError> {
    let message = "channels is a non-empty list of groups, mask, paint, layers or uvs";
    match args.get("channels") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(a)) if !a.is_empty() => {
            let mut out: Vec<&'static str> = Vec::new();
            for v in a {
                let name = v.as_str().and_then(|x| SET_CHANNELS.iter().copied().find(|c| *c == x)).ok_or_else(|| bad(message))?;
                if !out.contains(&name) {
                    out.push(name);
                }
            }
            Ok(Some(out))
        }
        Some(_) => Err(bad(message)),
    }
}

fn clip_channel(args: &Value, required: bool) -> Result<Option<&'static str>, CommandError> {
    match args.get("channel") {
        None | Some(Value::Null) if !required => Ok(None),
        Some(Value::String(c)) => ["groups", "mask", "paint"].into_iter().find(|x| x == c).map(Some).ok_or_else(|| bad("channel is groups, mask or paint")),
        _ => Err(bad("channel is groups, mask or paint")),
    }
}

fn present(o: &Object) -> Vec<&'static str> {
    let flags = [o.mesh.has_groups(), o.mask.len() == o.mesh.vertex_count(), o.mesh.paint.len() == o.mesh.vertex_count() && o.mesh.has_paint(), !o.layers.list.is_empty(), !o.mesh.uvs.is_empty() && o.mesh.uvs.len() == o.mesh.indices.len()];
    SET_CHANNELS.iter().zip(flags).filter(|(_, f)| *f).map(|(c, _)| *c).collect()
}

fn sizes(set: &SelectionSet) -> Value {
    let mut out = Map::new();
    if let Some(g) = &set.groups {
        out.insert("groups".into(), json!(g.len()));
    }
    if let Some(m) = &set.mask {
        out.insert("mask".into(), json!(m.len()));
    }
    if let Some(p) = &set.paint {
        out.insert("paint".into(), json!(p.len()));
    }
    if let Some(l) = &set.strengths {
        out.insert("layers".into(), json!(l.len()));
    }
    if let Some(u) = &set.uvs {
        out.insert("uvs".into(), json!(u.len()));
    }
    Value::Object(out)
}

fn fits(o: &Object, set: &SelectionSet, channel: &str) -> bool {
    let (v, t) = (o.mesh.vertex_count(), o.mesh.triangle_count());
    match channel {
        "groups" => set.faces == t,
        "mask" | "paint" => set.vertices == v,
        "layers" => set.vertices == v && set.strengths.as_ref().is_some_and(|s| s.len() == o.layers.list.len()),
        _ => set.faces == t && set.uvs.as_ref().is_some_and(|u| u.len() == o.mesh.indices.len()),
    }
}

fn commit(s: &mut Session, id: ObjectId, label: &str, mut state: ChannelState) -> Result<Value, CommandError> {
    let o = s.scene.get_mut(id).expect("checked");
    o.swap_channels(&mut state).map_err(|e| CommandError::new("topology_changed", e))?;
    s.history.push(Edit::Channels { label: label.into(), object: id, state });
    Ok(summary(s.scene.get(id).expect("checked")))
}

pub(crate) fn save_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": ID},
            "name": {"type": "string", "minLength": 1, "maxLength": 64, "description": "The set's name; saving under an existing name replaces that set."},
            "channels": {"type": "array", "minItems": 1, "uniqueItems": true, "items": {"type": "string", "enum": ["groups", "mask", "paint", "layers", "uvs"]}, "description": "What to keep: face groups, the mask, vertex paint (the composited colour, roughness, metalness, opacity and glow), sculpt layer strengths and UVs. All the channels the object has when omitted."}
        },
        "required": ["name"],
        "additionalProperties": false
    })
}

pub(crate) fn selection_set_save(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let name = set_name(args)?;
    let asked = channel_list(args)?;
    let o = s.scene.get(id).expect("checked");
    let have = present(o);
    let chosen: Vec<&'static str> = match asked {
        Some(list) => {
            if let Some(missing) = list.iter().find(|c| !have.contains(c)) {
                return Err(CommandError::new("not_present", format!("the object has no {missing} to save; it has: {}", have.join(", "))));
            }
            list
        }
        None => have,
    };
    let has = |c: &str| chosen.contains(&c);
    let set = SelectionSet {
        name: name.clone(),
        vertices: o.mesh.vertex_count(),
        faces: o.mesh.triangle_count(),
        groups: has("groups").then(|| o.mesh.groups.clone()),
        mask: has("mask").then(|| o.mask.clone()),
        paint: has("paint").then(|| o.mesh.paint.clone()),
        strengths: has("layers").then(|| o.layers.list.iter().map(|l| l.strength).collect()),
        uvs: has("uvs").then(|| o.mesh.uvs.clone()),
    };
    let result = json!({"name": name, "channels": set.channels(), "sizes": sizes(&set), "vertices": set.vertices, "faces": set.faces});
    let o = s.scene.get_mut(id).expect("checked");
    let replaced = match o.selection_sets.iter().position(|x| x.name == name) {
        Some(at) => {
            o.selection_sets[at] = set;
            true
        }
        None if o.selection_sets.len() >= LIMIT => return Err(CommandError::new("too_many_sets", format!("an object keeps at most {LIMIT} selection sets; delete one first"))),
        None => {
            o.selection_sets.push(set);
            false
        }
    };
    let total = o.selection_sets.len();
    let mut out = result;
    out["object"] = json!(id);
    out["replaced"] = json!(replaced);
    out["sets"] = json!(total);
    Ok(out)
}

pub(crate) fn restore_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": ID},
            "name": {"type": "string", "minLength": 1, "maxLength": 64, "description": "The set to restore."},
            "channels": {"type": "array", "minItems": 1, "uniqueItems": true, "items": {"type": "string", "enum": ["groups", "mask", "paint", "layers", "uvs"]}, "description": "Restore only these channels, which the set must hold. Every channel the set holds when omitted."}
        },
        "required": ["name"],
        "additionalProperties": false
    })
}

pub(crate) fn selection_set_restore(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let name = set_name(args)?;
    let asked = channel_list(args)?;
    let o = s.scene.get(id).expect("checked");
    let set = o.selection_sets.iter().find(|x| x.name == name).ok_or_else(|| CommandError::new("not_found", format!("the object has no selection set named {name}; selection_set_list names them")))?;
    let stored = set.channels();
    let chosen = match asked {
        Some(list) => {
            if let Some(missing) = list.iter().find(|c| !stored.contains(c)) {
                return Err(CommandError::new("not_present", format!("the set {name} holds no {missing}; it holds: {}", stored.join(", "))));
            }
            list
        }
        None => stored,
    };
    let blocked: Vec<&str> = chosen.iter().copied().filter(|c| !fits(o, set, c)).collect();
    if !blocked.is_empty() {
        return Err(CommandError::new(
            "topology_changed",
            format!(
                "the set {name} was saved with {} vertices and {} faces, but the object now has {} vertices and {} faces and {} layers; {} cannot be restored. Nothing was changed.",
                set.vertices,
                set.faces,
                o.mesh.vertex_count(),
                o.mesh.triangle_count(),
                o.layers.list.len(),
                blocked.join(", ")
            ),
        ));
    }
    let has = |c: &str| chosen.contains(&c);
    let mut state = ChannelState::default();
    if has("groups") {
        state.groups = set.groups.clone();
    }
    if has("mask") {
        state.mask = set.mask.clone();
    }
    if has("paint") {
        state.paint = set.paint.clone().map(|p| (p, PaintLayers::default()));
    }
    if has("uvs") {
        state.uvs = set.uvs.clone();
    }
    if has("layers") {
        let strengths = set.strengths.as_deref().unwrap_or_default();
        let mut layers = o.layers.clone();
        let mut positions = o.mesh.positions.clone();
        for (l, &new) in layers.list.iter_mut().zip(strengths) {
            let by = new - l.strength;
            if by != 0.0 {
                for (p, d) in positions.iter_mut().zip(&l.delta) {
                    for k in 0..3 {
                        p[k] += d[k] * by;
                    }
                }
            }
            l.strength = new;
        }
        state.layers = Some((layers, positions));
    }
    let restored: Vec<&str> = chosen.clone();
    let object = commit(s, id, "Restore Selection Set", state)?;
    Ok(json!({"object": object, "name": name, "restored": restored}))
}

pub(crate) fn name_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": ID},
            "name": {"type": "string", "minLength": 1, "maxLength": 64, "description": "The set to delete."}
        },
        "required": ["name"],
        "additionalProperties": false
    })
}

pub(crate) fn selection_set_delete(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let name = set_name(args)?;
    let o = s.scene.get_mut(id).expect("checked");
    let at = o.selection_sets.iter().position(|x| x.name == name).ok_or_else(|| CommandError::new("not_found", format!("the object has no selection set named {name}; selection_set_list names them")))?;
    o.selection_sets.remove(at);
    Ok(json!({"object": id, "deleted": name, "sets": o.selection_sets.len()}))
}

pub(crate) fn list_schema() -> Value {
    json!({"type": "object", "properties": {"id": {"type": "integer", "minimum": 1, "description": ID}}, "required": [], "additionalProperties": false})
}

pub(crate) fn selection_set_list(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let o = s.scene.get(id).expect("checked");
    let sets: Vec<Value> = o
        .selection_sets
        .iter()
        .map(|set| {
            let stored = set.channels();
            let ok: Vec<&str> = stored.iter().copied().filter(|c| fits(o, set, c)).collect();
            json!({"name": set.name, "channels": stored, "sizes": sizes(set), "vertices": set.vertices, "faces": set.faces, "restorable": ok.len() == stored.len(), "restorable_channels": ok})
        })
        .collect();
    Ok(json!({"object": id, "vertices": o.mesh.vertex_count(), "faces": o.mesh.triangle_count(), "sets": sets}))
}

pub(crate) fn copy_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": ID},
            "channel": {"type": "string", "enum": ["groups", "mask", "paint"], "description": "What to copy: the face groups (per face), the mask or the vertex paint (per vertex, as composited)."}
        },
        "required": ["channel"],
        "additionalProperties": false
    })
}

pub(crate) fn channel_copy(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let channel = clip_channel(args, true)?.expect("required");
    let o = s.scene.get(id).expect("checked");
    let clip = match channel {
        "groups" if o.mesh.has_groups() => Clip::Groups(o.mesh.groups.clone()),
        "groups" => return Err(CommandError::new("no_groups", "the object has no face groups to copy")),
        "mask" => Clip::Mask(o.mask.clone()),
        _ if o.mesh.has_paint() && o.mesh.paint.len() == o.mesh.vertex_count() => Clip::Paint(o.mesh.paint.clone()),
        _ => return Err(CommandError::new("no_paint", "the object has no paint to copy")),
    };
    let count = clip.len();
    s.clipboard = Some(clip);
    Ok(json!({"object": id, "channel": channel, "count": count}))
}

pub(crate) fn paste_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": ID},
            "channel": {"type": "string", "enum": ["groups", "mask", "paint"], "description": "The channel the clipboard must hold; whatever was copied when omitted."}
        },
        "required": [],
        "additionalProperties": false
    })
}

pub(crate) fn channel_paste(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let asked = clip_channel(args, false)?;
    let clip = s.clipboard.clone().ok_or_else(|| CommandError::new("clipboard_empty", "nothing has been copied; channel_copy first"))?;
    if let Some(c) = asked.filter(|c| *c != clip.channel()) {
        return Err(CommandError::new("wrong_channel", format!("the clipboard holds {}, not {c}", clip.channel())));
    }
    let o = s.scene.get(id).expect("checked");
    let (need, unit) = match clip {
        Clip::Groups(_) => (o.mesh.triangle_count(), "faces"),
        _ => (o.mesh.vertex_count(), "vertices"),
    };
    if clip.len() != need {
        return Err(CommandError::new("count_mismatch", format!("the clipboard holds {} {} of {}, but the object has {need} {unit}", clip.len(), unit, clip.channel())));
    }
    let channel = clip.channel();
    let mut state = ChannelState::default();
    let label = match clip {
        Clip::Groups(g) => {
            state.groups = Some(g);
            "Paste Polygroups"
        }
        Clip::Mask(m) => {
            state.mask = Some(m);
            "Paste Mask"
        }
        Clip::Paint(p) => {
            state.paint = Some((p, PaintLayers::default()));
            "Paste Polypaint"
        }
    };
    let object = commit(s, id, label, state)?;
    Ok(json!({"object": object, "channel": channel}))
}

pub(crate) fn clear_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "id": {"type": "integer", "minimum": 1, "description": ID},
            "channel": {"type": "string", "enum": ["groups", "mask", "paint"], "description": "groups puts every face in group 0; mask sets every vertex to 0; paint removes all vertex paint and the paint layer stack, back to unpainted."}
        },
        "required": ["channel"],
        "additionalProperties": false
    })
}

pub(crate) fn channel_clear(s: &mut Session, args: &Value) -> Result<Value, CommandError> {
    let id = target(s, args)?;
    let channel = clip_channel(args, true)?.expect("required");
    let o = s.scene.get(id).expect("checked");
    let mut state = ChannelState::default();
    let label = match channel {
        "groups" => {
            if o.mesh.has_groups() && o.mesh.groups.iter().any(|&g| g != 0) {
                state.groups = Some(vec![0; o.mesh.triangle_count()]);
            }
            "Clear Polygroups"
        }
        "mask" => {
            if o.mask.iter().any(|&m| m != 0.0) {
                state.mask = Some(vec![0.0; o.mesh.vertex_count()]);
            }
            "Clear Mask"
        }
        _ => {
            if o.mesh.has_paint() || !o.paint_layers.is_empty() {
                state.paint = Some((Vec::new(), PaintLayers::default()));
            }
            "Clear Polypaint"
        }
    };
    if state.groups.is_none() && state.mask.is_none() && state.paint.is_none() {
        return Ok(json!({"object": summary(o), "channel": channel, "changed": false}));
    }
    let object = commit(s, id, label, state)?;
    Ok(json!({"object": object, "channel": channel, "changed": true}))
}
