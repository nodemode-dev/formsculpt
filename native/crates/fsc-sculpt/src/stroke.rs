use std::collections::HashMap;

use fsc_core::texture::{rasterize, Texture, TextureEdit, PAD};
use fsc_core::{paint_channel, LayerPaint, Mesh, Object, Paint, PAINT_LEN};
use glam::{Mat3, Vec2, Vec3};

use crate::alpha::Alpha;
use crate::brush::{BrushKind, BrushSettings};

#[derive(Debug, Clone, Default)]
pub struct VertexEdit {
    pub indices: Vec<u32>,
    pub before: Vec<[f32; 3]>,
    pub after: Vec<[f32; 3]>,
    pub mask_before: Vec<f32>,
    pub mask_after: Vec<f32>,
    pub paint_before: Vec<Paint>,
    pub paint_after: Vec<Paint>,
    /// With paint layers: the target painted (a layer index, or None for the base) and its values per index.
    pub layer: Option<(Option<usize>, Vec<LayerPaint>, Vec<LayerPaint>)>,
}

impl VertexEdit {
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// Records the edit's shape, paint and mask changes into layer `at` (`sign` -1 takes them back out).
    pub fn record_into_layer(&self, object: &mut Object, at: usize, sign: f32) {
        object.record_into_layer(at, &self.indices, &self.before, &self.after, sign);
        object.record_paint_into_layer(at, &self.indices, &self.paint_before, &self.paint_after, sign);
        object.record_mask_into_layer(at, &self.indices, &self.mask_before, &self.mask_after, sign);
    }

    pub fn apply(&self, object: &mut Object, forward: bool) -> Vec<u32> {
        let (pos, mask) = if forward { (&self.after, &self.mask_after) } else { (&self.before, &self.mask_before) };
        let paint = if forward { &self.paint_after } else { &self.paint_before };
        if !paint.is_empty() {
            object.mesh.ensure_paint();
        }
        for (k, &v) in self.indices.iter().enumerate() {
            object.mesh.positions[v as usize] = pos[k];
            object.mask[v as usize] = mask[k];
            if let Some(p) = paint.get(k) {
                object.mesh.paint[v as usize] = *p;
            }
        }
        if let Some((target, before, after)) = &self.layer {
            if object.layered() {
                let values = if forward { after } else { before };
                for (k, &v) in self.indices.iter().enumerate() {
                    object.set_paint_target(*target, v, values[k]);
                }
            }
        }
        let mut ring = Vec::new();
        object.topology.ring_vertices(&self.indices, &mut ring);
        object.topology.update_normals(&mut object.mesh, &ring);
        object.refit_vertices(&self.indices);
        object.revision += 1;
        ring
    }
}

type Captured = Vec<(u32, f32, Vec3)>;

/// How free a vertex is to change: 1 minus its mask, and 0 when every face around it is hidden (hidden areas are held
/// like a full mask). `hidden` is whether the mesh hides anything, checked once per dab.
fn free(o: &Object, v: u32, hidden: bool) -> f32 {
    if hidden && o.topology.faces_of(v).iter().all(|&t| o.mesh.hidden[t as usize]) {
        return 0.0;
    }
    1.0 - o.mask[v as usize]
}

fn xorshift(state: &mut u64) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state >> 40) as f32 / (1u64 << 24) as f32
}

/// The falloff weight of each vertex of `found` around `center`, with the weighted sum of normals and the weighted mean position.
fn weigh(object: &Object, found: &[u32], center: Vec3, r: f32, s: &BrushSettings) -> (Vec<f32>, Vec3, Vec3) {
    let (mut area_n, mut area_c, mut wsum) = (Vec3::ZERO, Vec3::ZERO, 0.0);
    let weights = found
        .iter()
        .map(|&v| {
            let p = object.mesh.position(v);
            let w = s.falloff.weight(p.distance(center) / r);
            area_n += Vec3::from_array(object.mesh.normals[v as usize]) * w;
            area_c += p * w;
            wsum += w;
            w
        })
        .collect();
    (weights, area_n, if wsum > 0.0 { area_c / wsum } else { center })
}

/// The weighted sum of normals and mean position taken over `area_sample` times the radius (the gathered set, or a wider gather above 1).
fn sampled_area(object: &Object, found: &[u32], center: Vec3, r: f32, s: &BrushSettings, fallback: (Vec3, Vec3)) -> (Vec3, Vec3) {
    let rr = r * s.area_sample;
    let mut wide = Vec::new();
    let set: &[u32] = if s.area_sample > 1.0 {
        object.bvh.vertices_in_sphere(&object.mesh, center, rr, &mut wide);
        &wide
    } else {
        found
    };
    let (mut n, mut c, mut wsum) = (Vec3::ZERO, Vec3::ZERO, 0.0);
    for &v in set {
        let p = object.mesh.position(v);
        let d = p.distance(center);
        if d > rr {
            continue;
        }
        let w = s.falloff.weight(d / rr);
        n += Vec3::from_array(object.mesh.normals[v as usize]) * w;
        c += p * w;
        wsum += w;
    }
    if wsum > 0.0 && n != Vec3::ZERO {
        (n, c / wsum)
    } else {
        fallback
    }
}

struct Grab {
    anchor: Vec3,
    plane_normal: Vec3,
    kelvin: bool,
    copies: Vec<(Mat3, Captured)>,
}

pub struct Stroke {
    pub settings: BrushSettings,
    transforms: Vec<Mat3>,
    last: Option<Vec3>,
    lazy_point: Option<Vec3>,
    before: HashMap<u32, ([f32; 3], f32, Paint)>,
    changed: Vec<u32>,
    grab: Option<Grab>,
    found: Vec<u32>,
    snapshot: Option<(Mesh, Vec<f32>)>,
    topology_dirty: bool,
    topology_changed: bool,
    /// Planar: the plane (centre, normal) taken at the first dab, one per symmetry copy.
    planes: Vec<Option<(Vec3, Vec3)>>,
    /// Snake hook: the previous dab centre, before symmetry.
    snake_prev: Option<Vec3>,
    /// The brush alpha, if any, and the previous dab centre per symmetry copy (roll turns the alpha along the stroke).
    alpha: Option<std::sync::Arc<Alpha>>,
    prev_dab: Vec<Option<Vec3>>,
    /// Smudge: the colour carried from the previous dab.
    smudge: Option<Vec3>,
    /// Spray: a seeded generator, and the current dab's extra turn (radians) and share of the strength.
    rng: u64,
    /// Jitter: its own generator state, and the current dab's radius and strength scales.
    jrng: u64,
    dab_mul: (f32, f32),
    jitter: (f32, f32),
    /// The area normal under the last unmirrored dab, the plane spray scatters in.
    last_normal: Vec3,
    tilt: f32,
    /// With paint layers: the paint target's values before the stroke, and which target that was.
    layer_before: HashMap<u32, LayerPaint>,
    paint_target: Option<Option<usize>>,
    /// Layer brush: per vertex the offset accumulated so far this stroke and the direction it rises along.
    layer_acc: HashMap<u32, (f32, Vec3)>,
    /// Layer reset: per vertex the active layer's contribution at first touch and the share of it still left.
    reset_left: HashMap<u32, (Vec3, f32)>,
    /// Nudge: the surface as it was when the stroke began, which slid vertices are put back onto.
    nudge_surface: Option<(Mesh, fsc_core::Bvh)>,
    /// Set when a layer reset ran with no active sculpt layer.
    pub no_active_layer: bool,
    pub dabs: u32,
    /// The view direction of the unmirrored copy: given, from the latest ray, or taken from the first dab's normal.
    view: Option<Vec3>,
    /// Keep sharp: per vertex 0 (on a sharp edge), 0.5 (next to one) or 1, captured before the first dab.
    sharp: Option<Vec<f32>>,
    /// Protect group: the group under each symmetry copy's first dab.
    group_at: Vec<Option<u32>>,
    /// Accumulate off: per vertex the largest move one dab gave it.
    dab_max: HashMap<u32, f32>,
    /// Texture painting: each texel's paint before the stroke first touched it.
    tex_before: HashMap<u32, Paint>,
}

pub enum StrokeEdit {
    Texels(TextureEdit),
    Vertices(VertexEdit),
    Topology { mesh: Mesh, mask: Vec<f32> },
    Nothing,
}

impl std::fmt::Debug for Stroke {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stroke").field("settings", &self.settings).field("dabs", &self.dabs).finish()
    }
}

impl Stroke {
    pub fn new(settings: BrushSettings) -> Stroke {
        let transforms = settings.symmetry.transforms();
        let planes = vec![None; transforms.len()];
        Stroke {
            settings,
            prev_dab: vec![None; transforms.len()],
            alpha: None,
            smudge: None,
            rng: 0x9e37_79b9_7f4a_7c15,
            jrng: {
                let mut x = settings.jitter_seed.wrapping_add(1).wrapping_mul(0x9e37_79b9_7f4a_7c15);
                x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
                x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
                (x ^ (x >> 31)).max(1)
            },
            dab_mul: (1.0, 1.0),
            jitter: (0.0, 1.0),
            last_normal: Vec3::Y,
            tilt: 0.0,
            layer_before: HashMap::new(),
            paint_target: None,
            layer_acc: HashMap::new(),
            reset_left: HashMap::new(),
            nudge_surface: None,
            no_active_layer: false,
            view: settings.view_dir.and_then(|d| Vec3::from_array(d).try_normalize()),
            sharp: None,
            group_at: vec![None; transforms.len()],
            dab_max: HashMap::new(),
            tex_before: HashMap::new(),
            planes,
            snake_prev: None,
            transforms,
            last: None,
            lazy_point: None,
            before: HashMap::new(),
            changed: Vec::new(),
            grab: None,
            found: Vec::new(),
            snapshot: None,
            topology_dirty: false,
            topology_changed: false,
            dabs: 0,
        }
    }

    /// A uniform number in 0..1 from a seeded xorshift, so a spray stroke is the same on every replay.
    fn random(&mut self) -> f32 {
        xorshift(&mut self.rng)
    }

    /// The same generator on its own state, seeded by `jitter_seed`, so jitter and spray never disturb each other.
    fn jitter_random(&mut self) -> f32 {
        xorshift(&mut self.jrng)
    }

    /// A dab's radius: pressure, then the jitter scale, then the snap to a multiple of `snap_radius` (at least one step).
    fn dab_radius(&self, pressure: f32) -> f32 {
        let s = &self.settings;
        let r = if s.pressure_size { s.radius * pressure.max(0.1) } else { s.radius } * self.dab_mul.0;
        if s.snap_radius > 0.0 {
            (r / s.snap_radius).round().max(1.0) * s.snap_radius
        } else {
            r
        }
    }

    /// Shapes every dab by a grayscale alpha (multiplied with the falloff).
    /// Pen tilt: the direction the pen leans, in radians from the screen's up, turns the alpha of the dabs that
    /// follow (0 for a mouse or an upright pen).
    pub fn set_tilt(&mut self, radians: f32) {
        self.tilt = if radians.is_finite() { radians } else { 0.0 };
    }

    pub fn set_alpha(&mut self, alpha: Option<std::sync::Arc<Alpha>>) {
        self.alpha = alpha;
    }

    pub fn sample_ray(&mut self, object: &mut Object, origin: Vec3, dir: Vec3, pressure: f32) -> bool {
        let dir = dir.normalize_or(Vec3::NEG_Z);
        self.view = Some(dir);
        self.capture(object);
        if self.settings.kind.drags() {
            if let Some(grab) = &self.grab {
                let denom = dir.dot(grab.plane_normal);
                if denom.abs() < 1e-6 {
                    return true;
                }
                let t = (grab.anchor - origin).dot(grab.plane_normal) / denom;
                let target = origin + dir * t;
                self.apply_grab(object, target - grab.anchor, pressure);
                return true;
            }
            let Some(hit) = object.bvh.raycast(&object.mesh, origin, dir) else { return false };
            self.begin_grab(object, hit.point, -dir);
            return true;
        }
        if self.settings.kind == BrushKind::SnakeHook {
            // Once the hook has started, follow the pointer on the plane through the last dab facing the camera, so
            // the pull continues off the surface as grab does.
            if let Some(prev) = self.last {
                let t = (prev - origin).dot(dir);
                self.sample_point(object, origin + dir * t, pressure);
                return true;
            }
        }
        let Some(hit) = object.bvh.raycast(&object.mesh, origin, dir) else {
            self.last = None;
            return false;
        };
        self.sample_point(object, hit.point, pressure);
        true
    }

    pub fn grab_between(&mut self, object: &mut Object, from: Vec3, to: Vec3, pressure: f32) {
        self.capture(object);
        self.begin_grab(object, from, Vec3::Z);
        self.apply_grab(object, to - from, pressure);
    }

    pub fn sample_point(&mut self, object: &mut Object, point: Vec3, pressure: f32) {
        self.capture(object);
        // Lazy rope: the brush trails the pointer on a rope of lazy times three radii and moves only when the rope
        // pulls taut, so hand jitter inside the rope never reaches the surface and lines come out steady.
        let point = match (self.lazy_point, self.settings.lazy) {
            (Some(prev), lazy) if lazy > 0.0 => {
                let rope = lazy.clamp(0.0, 0.95) * self.settings.radius * 3.0;
                let d = prev.distance(point);
                if d <= rope {
                    return;
                }
                prev + (point - prev) * (1.0 - rope / d)
            }
            _ => point,
        };
        self.lazy_point = Some(point);
        let step = (self.settings.radius * self.settings.spacing).max(1e-6);
        let mut centers = Vec::new();
        match self.last {
            None => centers.push(point),
            Some(prev) => {
                let dist = prev.distance(point);
                let n = (dist / step).floor() as u32;
                for i in 1..=n {
                    centers.push(prev.lerp(point, (i as f32 * step) / dist));
                }
            }
        }
        if let Some(&last) = centers.last() {
            self.last = Some(last);
        } else if self.last.is_none() {
            self.last = Some(point);
        }
        if let (Some(frac), false) = (self.settings.dyntopo, centers.is_empty()) {
            self.refine(object, &centers, frac);
        }
        // Pen pressure through its curve: a gamma above 1 needs a firmer press for the same effect.
        let pressure = pressure.clamp(0.0, 1.0).powf(self.settings.pressure_gamma.clamp(0.2, 5.0));
        for c in centers {
            // Spray scatters each dab around the path (in the plane across the surface normal there), turns its
            // alpha at random and gives it a random share of the strength.
            if self.settings.kind == BrushKind::Stamp && self.dabs > 0 {
                break;
            }
            let c = if self.settings.spray > 0.0 && !matches!(self.settings.kind, BrushKind::SnakeHook | BrushKind::Nudge) {
                let (a, d, t, k) = (self.random(), self.random(), self.random(), self.random());
                let n = self.last_normal;
                let u = n.any_orthonormal_vector();
                let w = n.cross(u);
                let ang = a * std::f32::consts::TAU;
                self.jitter = (t * std::f32::consts::TAU, 0.4 + 0.6 * k);
                c + (u * ang.cos() + w * ang.sin()) * (d.sqrt() * self.settings.spray.min(2.0) * self.settings.radius)
            } else {
                c
            };
            let s = self.settings;
            let c = if s.jitter_position > 0.0 || s.jitter_radius > 0.0 || s.jitter_strength > 0.0 {
                let (a, d, rr, ss) = (self.jitter_random(), self.jitter_random(), self.jitter_random(), self.jitter_random());
                self.dab_mul = (1.0 + (rr * 2.0 - 1.0) * s.jitter_radius, 1.0 + (ss * 2.0 - 1.0) * s.jitter_strength);
                if s.jitter_position > 0.0 && !matches!(s.kind, BrushKind::SnakeHook | BrushKind::Nudge) {
                    let n = self.last_normal;
                    let u = n.any_orthonormal_vector();
                    let w = n.cross(u);
                    let ang = a * std::f32::consts::TAU;
                    c + (u * ang.cos() + w * ang.sin()) * (d.sqrt() * s.jitter_position * s.radius)
                } else {
                    c
                }
            } else {
                c
            };
            if self.settings.kind == BrushKind::SnakeHook {
                if let Some(prev) = self.snake_prev {
                    for k in 0..self.transforms.len() {
                        let m = self.transforms[k];
                        self.snake_dab(object, k, m * prev, m * (c - prev));
                    }
                    self.dabs += 1;
                }
                self.snake_prev = Some(c);
                continue;
            }
            if self.settings.kind == BrushKind::Nudge {
                if let Some(prev) = self.snake_prev {
                    for k in 0..self.transforms.len() {
                        let m = self.transforms[k];
                        self.nudge_dab(object, k, m * ((prev + c) * 0.5), m * (c - prev), pressure.clamp(0.0, 1.0));
                    }
                    self.dabs += 1;
                }
                self.snake_prev = Some(c);
                continue;
            }
            for k in 0..self.transforms.len() {
                let m = self.transforms[k];
                self.dab(object, k, m * c, pressure.clamp(0.0, 1.0));
            }
            self.dabs += 1;
        }
        if self.settings.kind.moves_vertices() && !self.changed.is_empty() {
            object.refit_vertices(&self.changed);
        }
    }

    fn refine(&mut self, object: &mut Object, centers: &[Vec3], frac: f32) {
        if matches!(self.settings.kind, BrushKind::Mask | BrushKind::Grab | BrushKind::Elastic | BrushKind::Paint | BrushKind::Smudge | BrushKind::LayerReset | BrushKind::Layer) {
            return;
        }
        if self.snapshot.is_none() {
            self.snapshot = Some((object.mesh.clone(), object.mask.clone()));
        }
        let first = centers[0];
        let last = *centers.last().expect("non-empty");
        let mid = (first + last) * 0.5;
        let reach = self.settings.radius + first.distance(last) * 0.5;
        let max_edge = self.settings.dyntopo_size.unwrap_or(self.settings.radius * frac.clamp(0.05, 1.0)).max(1e-5);
        let mut splits = 0;
        let (passes, relaxes) = self.settings.dyntopo_mode.passes();
        let method = self.settings.dyntopo_method;
        let protect = self.settings.dyntopo_protect_mask;
        for k in 0..self.transforms.len() {
            let m = self.transforms[k];
            if method != crate::brush::DyntopoMethod::Collapse {
                splits += fsc_topo::refine_sphere_protected(&mut object.mesh, &mut object.mask, m * mid, reach, max_edge, passes, protect);
            }
            if method != crate::brush::DyntopoMethod::Subdivide {
                splits += fsc_topo::collapse_sphere(&mut object.mesh, &mut object.mask, m * mid, reach, max_edge * 0.4);
            }
        }
        if splits > 0 {
            object.mesh.uvs.clear();
            object.mesh.groups.clear();
            object.mesh.hidden.clear();
            object.mesh.quads = false;
            object.retopologize();
            let mut region = Vec::new();
            for k in 0..self.transforms.len() {
                let mut found = Vec::new();
                object.bvh.vertices_in_sphere(&object.mesh, self.transforms[k] * mid, reach, &mut found);
                region.extend(found);
            }
            region.sort_unstable();
            region.dedup();
            for _ in 0..relaxes {
                relax_tangential(object, &region, 0.5);
            }
            let mut ring = Vec::new();
            object.topology.ring_vertices(&region, &mut ring);
            object.topology.update_normals(&mut object.mesh, &ring);
            object.refit_vertices(&region);
            self.topology_dirty = true;
            self.topology_changed = true;
        }
    }

    /// Captures what the filters need from the mesh before the stroke has touched it (the sharp-edge map).
    fn capture(&mut self, object: &Object) {
        let deg = self.settings.keep_sharp;
        if deg <= 0.0 || self.sharp.is_some() {
            return;
        }
        let m = &object.mesh;
        let normals: Vec<Vec3> = m
            .indices
            .chunks_exact(3)
            .map(|t| {
                let (a, b, c) = (m.position(t[0]), m.position(t[1]), m.position(t[2]));
                (b - a).cross(c - a).normalize_or_zero()
            })
            .collect();
        let cos = deg.to_radians().cos();
        let mut held = vec![false; m.vertex_count()];
        let mut edges: HashMap<(u32, u32), usize> = HashMap::new();
        for (t, tri) in m.indices.chunks_exact(3).enumerate() {
            for e in 0..3 {
                let (a, b) = (tri[e], tri[(e + 1) % 3]);
                match edges.get(&(a.min(b), a.max(b))) {
                    Some(&u) => {
                        if normals[t] != Vec3::ZERO && normals[u] != Vec3::ZERO && normals[t].dot(normals[u]) < cos {
                            held[a as usize] = true;
                            held[b as usize] = true;
                        }
                    }
                    None => {
                        edges.insert((a.min(b), a.max(b)), t);
                    }
                }
            }
        }
        let state = (0..m.vertex_count() as u32)
            .map(|v| {
                if held[v as usize] {
                    0.0
                } else if object.topology.neighbors_of(v).iter().any(|&u| held[u as usize]) {
                    0.5
                } else {
                    1.0
                }
            })
            .collect();
        self.sharp = Some(state);
    }

    /// The stroke filters as a weight multiplier per vertex of `found` (None when no filter is on): front facing,
    /// connected only, face groups, keep sharp and depth.
    fn filter(&mut self, object: &Object, copy: usize, found: &[u32], center: Vec3, r: f32) -> Option<Vec<f32>> {
        let s = self.settings;
        let groups = s.protect_group || s.protect_groups.len > 0;
        if found.is_empty() || !(s.front_facing || s.connected_only || groups || s.keep_sharp > 0.0 || s.depth > 0.0) {
            return None;
        }
        let m = &object.mesh;
        let mut mult = vec![1.0f32; found.len()];
        let nearest = found.iter().copied().min_by(|&a, &b| m.position(a).distance_squared(center).total_cmp(&m.position(b).distance_squared(center))).expect("non-empty");
        let ground = Vec3::from_array(m.normals[nearest as usize]);
        if s.front_facing {
            if self.view.is_none() {
                self.view = Some(self.transforms[copy].inverse() * -ground);
            }
            let look = self.transforms[copy] * self.view.expect("set above");
            for (k, &v) in found.iter().enumerate() {
                let facing = -Vec3::from_array(m.normals[v as usize]).dot(look);
                mult[k] *= (facing / 0.2).clamp(0.0, 1.0);
            }
        }
        if s.connected_only && !found.is_empty() {
            let at: HashMap<u32, usize> = found.iter().enumerate().map(|(k, &v)| (v, k)).collect();
            let dist = |k: usize| m.position(found[k]).distance_squared(center);
            let start = (0..found.len()).min_by(|&a, &b| dist(a).total_cmp(&dist(b))).expect("non-empty");
            let mut seen = vec![false; found.len()];
            seen[start] = true;
            let mut stack = vec![start];
            while let Some(k) = stack.pop() {
                for &u in object.topology.neighbors_of(found[k]) {
                    if let Some(&j) = at.get(&u) {
                        if !seen[j] {
                            seen[j] = true;
                            stack.push(j);
                        }
                    }
                }
            }
            for (k, &reached) in seen.iter().enumerate() {
                if !reached {
                    mult[k] = 0.0;
                }
            }
        }
        if groups && m.has_groups() {
            if s.protect_group && self.group_at[copy].is_none() {
                let nearest = found.iter().copied().min_by(|&a, &b| m.position(a).distance_squared(center).total_cmp(&m.position(b).distance_squared(center)));
                let face_dist = |t: u32| {
                    let i = &m.indices[3 * t as usize..3 * t as usize + 3];
                    ((m.position(i[0]) + m.position(i[1]) + m.position(i[2])) / 3.0).distance_squared(center)
                };
                let face = nearest.and_then(|n| object.topology.faces_of(n).iter().copied().min_by(|&a, &b| face_dist(a).total_cmp(&face_dist(b))));
                self.group_at[copy] = face.map(|t| m.groups[t as usize]);
            }
            let own = if s.protect_group { self.group_at[copy] } else { None };
            for (k, &v) in found.iter().enumerate() {
                let faces = object.topology.faces_of(v);
                if faces.is_empty() {
                    continue;
                }
                if let Some(g) = own {
                    let inside = faces.iter().filter(|&&t| m.groups[t as usize] == g).count();
                    mult[k] *= if inside == 0 {
                        0.0
                    } else if inside == faces.len() {
                        1.0
                    } else {
                        0.5
                    };
                }
                if s.protect_groups.len > 0 && faces.iter().all(|&t| s.protect_groups.contains(m.groups[t as usize])) {
                    mult[k] = 0.0;
                }
            }
        }
        if let Some(sharp) = &self.sharp {
            for (k, &v) in found.iter().enumerate() {
                mult[k] *= sharp.get(v as usize).copied().unwrap_or(1.0);
            }
        }
        if s.depth > 0.0 {
            let (mut sum_n, mut sum_c, mut wsum) = (Vec3::ZERO, Vec3::ZERO, 0.0);
            for &v in found {
                let n = Vec3::from_array(m.normals[v as usize]);
                if n.dot(ground) > 0.5 {
                    let p = m.position(v);
                    let w = s.falloff.weight(p.distance(center) / r);
                    sum_n += n * w;
                    sum_c += p * w;
                    wsum += w;
                }
            }
            let an = sum_n.normalize_or(ground);
            let ac = if wsum > 0.0 { sum_c / wsum } else { m.position(nearest) };
            for (k, &v) in found.iter().enumerate() {
                let below = (ac - m.position(v)).dot(an);
                let t = ((below - 0.8 * s.depth) / (0.2 * s.depth)).clamp(0.0, 1.0);
                mult[k] *= 1.0 - t * t * (3.0 - 2.0 * t);
            }
        }
        Some(mult)
    }

    /// With accumulate off: holds a vertex's offset from its stroke-start position to the largest move any one dab gave it.
    fn cap_move(&mut self, v: u32, p: Vec3, moved: Vec3) -> Vec3 {
        if self.settings.accumulate || self.settings.kind == BrushKind::Layer {
            return moved;
        }
        let Some(&(start, ..)) = self.before.get(&v) else { return moved };
        let start = Vec3::from_array(start);
        let top = self.dab_max.entry(v).or_insert(0.0);
        *top = top.max(moved.distance(p));
        let offset = moved - start;
        let len = offset.length();
        if len > *top {
            start + offset * (*top / len)
        } else {
            moved
        }
    }

    pub fn take_topology_dirty(&mut self) -> bool {
        std::mem::take(&mut self.topology_dirty)
    }

    fn remember(&mut self, object: &Object, v: u32) {
        let paint = object.mesh.paint.get(v as usize).copied().unwrap_or(fsc_core::DEFAULT_PAINT);
        self.before.entry(v).or_insert((object.mesh.positions[v as usize], object.mask[v as usize], paint));
        if let Some(value) = object.paint_target(v) {
            self.layer_before.entry(v).or_insert(value);
            self.paint_target.get_or_insert(object.paint_layers.active);
        }
    }

    /// Moves the vertices around `from` by `delta`, weighted by falloff and held by the mask: dab after dab, the
    /// pulled area follows the stroke and stretches into a hook (add dyntopo to keep detail along it).
    fn snake_dab(&mut self, object: &mut Object, copy: usize, from: Vec3, delta: Vec3) {
        let s = self.settings;
        let mut found = std::mem::take(&mut self.found);
        let r = self.dab_radius(1.0);
        object.bvh.vertices_in_sphere(&object.mesh, from, r, &mut found);
        let mult = self.filter(object, copy, &found, from, r);
        // The core follows the pointer fully at any strength: a partial move would leave the pulled area behind the
        // brush, out of reach of the next dab, and the hook would stall. Strength widens the pulled area instead.
        let soft = 1.5 - s.strength.clamp(0.0, 1.0);
        let hid = object.mesh.has_hidden();
        for (k, &v) in found.iter().enumerate() {
            self.remember(object, v);
            let p = object.mesh.position(v);
            let w = s.falloff.weight(p.distance(from) / r).powf(soft).min(1.0) * free(object, v, hid) * mult.as_ref().map_or(1.0, |m| m[k]);
            object.mesh.positions[v as usize] = (p + delta * w).to_array();
        }
        self.finish_dab(object, &found);
        self.found = found;
    }

    /// Slides the vertices around `center` by `delta` with its component along each vertex normal removed, weighted by
    /// falloff, strength and the mask, then puts each back onto the surface the stroke started with along its normal,
    /// so the surface moves along itself and keeps its height.
    fn nudge_dab(&mut self, object: &mut Object, copy: usize, center: Vec3, delta: Vec3, pressure: f32) {
        let s = self.settings;
        let mut found = std::mem::take(&mut self.found);
        let (surface, tree) = self.nudge_surface.take().unwrap_or_else(|| (object.mesh.clone(), fsc_core::Bvh::build(&object.mesh)));
        let r = self.dab_radius(1.0);
        let lift = r * 0.25;
        object.bvh.vertices_in_sphere(&object.mesh, center, r, &mut found);
        let hid = object.mesh.has_hidden();
        let amount = s.strength * if s.pressure_strength { pressure } else { 1.0 } * 2.0 * self.dab_mul.1;
        let mult = self.filter(object, copy, &found, center, r);
        for (k, &v) in found.iter().enumerate() {
            self.remember(object, v);
            let p = object.mesh.position(v);
            let n = Vec3::from_array(object.mesh.normals[v as usize]);
            let w = (s.falloff.weight(p.distance(center) / r) * amount).min(1.0) * free(object, v, hid) * mult.as_ref().map_or(1.0, |m| m[k]);
            let mut q = p + (delta - n * delta.dot(n)) * w;
            if w > 0.0 {
                if let Some(hit) = tree.raycast(&surface, q + n * lift, -n).filter(|h| h.t < lift * 2.0) {
                    q += n * (hit.point - q).dot(n);
                }
            }
            object.mesh.positions[v as usize] = q.to_array();
        }
        self.nudge_surface = Some((surface, tree));
        self.finish_dab(object, &found);
        self.found = found;
    }

    fn dab(&mut self, object: &mut Object, copy: usize, center: Vec3, pressure: f32) {
        let s = self.settings;
        if s.texture && s.kind == BrushKind::Paint {
            self.texture_dab(object, copy, center, pressure);
            return;
        }
        if s.kind == BrushKind::LayerReset && object.layers.active.is_none_or(|a| a >= object.layers.list.len()) {
            self.no_active_layer = true;
            return;
        }
        let r = self.dab_radius(pressure);
        let mut center = center;
        let mut found = std::mem::take(&mut self.found);
        object.bvh.vertices_in_sphere(&object.mesh, center, r, &mut found);
        if found.is_empty() {
            self.found = found;
            return;
        }
        let sign = if s.invert { -1.0 } else { 1.0 };
        let hid = object.mesh.has_hidden();
        let (mut weights, mut area_n, mut area_c) = weigh(object, &found, center, r, &s);
        if s.area_sample != 1.0 {
            (area_n, area_c) = sampled_area(object, &found, center, r, &s, (area_n, area_c));
        }
        if s.offset != 0.0 {
            center += area_n.normalize_or(Vec3::Y) * (s.offset * r);
            object.bvh.vertices_in_sphere(&object.mesh, center, r, &mut found);
            if found.is_empty() {
                self.found = found;
                return;
            }
            (weights, area_n, area_c) = weigh(object, &found, center, r, &s);
            if s.area_sample != 1.0 {
                (area_n, area_c) = sampled_area(object, &found, center, r, &s, (area_n, area_c));
            }
        }
        let area_n = area_n.normalize_or(Vec3::Y);
        if copy == 0 {
            self.last_normal = area_n;
        }
        let compensation = if s.spacing_compensation { (s.spacing / 0.15).min(4.0) } else { 1.0 };
        let amount = s.strength * if s.pressure_strength { pressure } else { 1.0 } * self.jitter.1 * self.dab_mul.1 * compensation;
        // The alpha lies in the plane of the area under the brush. Its top points along `alpha_up` (mirrored with the
        // copy), or with roll along the stroke's direction since the previous dab, then turns by `alpha_angle`.
        let signed = s.alpha_mid > 0.0 && matches!(s.kind, BrushKind::Draw | BrushKind::Stamp | BrushKind::Inflate | BrushKind::Noise);
        let prev = self.prev_dab[copy].replace(center);
        // The stroke's direction in the surface since the previous dab. Crease and pinch pull only across it, toward
        // the line: pulling toward each dab's centre also bunched vertices along the line from dab to dab, and a
        // stroke came out as a row of pinches (a hatched groove) instead of one clean fold.
        let along = prev.and_then(|p| {
            let d = center - p;
            (d - area_n * d.dot(area_n)).try_normalize()
        });
        // Their pull toward the line also adds up over every dab that passes a point, so dense spacing collapsed the
        // vertices onto the line and folded their triangles into striations. Below the default spacing each dab pulls
        // in proportion, so a stroke pinches the same however finely it is sampled (spacing compensation, when on,
        // already scales the whole dab).
        let pull = if s.spacing_compensation { 1.0 } else { (s.spacing / 0.15).clamp(0.05, 1.0) };
        if s.kind == BrushKind::ClayStrips {
            // A square footprint in the surface's plane, one side along the stroke: each vertex is weighed by the larger
            // of its two distances along the square's axes instead of its distance from the centre. Corners beyond the
            // gathered sphere are left out, so the square's corners come out slightly rounded.
            let x_axis = along.unwrap_or_else(|| area_n.any_orthonormal_vector());
            let y_axis = area_n.cross(x_axis);
            for (k, &v) in found.iter().enumerate() {
                let d = object.mesh.position(v) - center;
                let reach = d.dot(x_axis).abs().max(d.dot(y_axis).abs()) / r;
                weights[k] = s.falloff.weight(reach);
            }
        }
        if let Some(alpha) = &self.alpha {
            let m = self.transforms[copy];
            let up_ref = match (s.roll, prev) {
                (true, Some(p)) if p.distance(center) > 1e-7 => center - p,
                _ => m * Vec3::from(s.alpha_up),
            };
            let y_axis = (up_ref - area_n * up_ref.dot(area_n)).try_normalize().unwrap_or_else(|| area_n.any_orthonormal_vector());
            let x_axis = y_axis.cross(area_n);
            for (k, &v) in found.iter().enumerate() {
                let d = object.mesh.position(v) - center;
                let (x, y) = crate::alpha::turn(d.dot(x_axis) / r, d.dot(y_axis) / r, s.alpha_angle + self.jitter.0 + self.tilt + s.alpha_rotation);
                let a = alpha.stamp(x, y, s.alpha_stretch, s.alpha_tile, s.alpha_invert, s.alpha_mid);
                weights[k] *= if signed { a } else { a.max(0.0) };
            }
        }
        if let Some(mult) = self.filter(object, copy, &found, center, r) {
            for (w, m) in weights.iter_mut().zip(mult) {
                *w *= m;
            }
        }
        for &v in &found {
            self.remember(object, v);
        }
        if s.kind == BrushKind::Smudge {
            // Smudge carries the colour picked up under the previous dab into this one, then picks up a mix of what
            // is here, so colours drag along the stroke.
            object.mesh.ensure_paint();
            let mut here = Vec3::ZERO;
            let mut total = 0.0;
            for (k, &v) in found.iter().enumerate() {
                let c = object.mesh.paint[v as usize];
                here += Vec3::new(c[0], c[1], c[2]) * weights[k];
                total += weights[k];
            }
            let here = if total > 0.0 { here / total } else { Vec3::splat(0.8) };
            if let Some(carried) = self.smudge {
                let mut target = fsc_core::DEFAULT_PAINT;
                target[..3].copy_from_slice(&carried.to_array());
                for (k, &v) in found.iter().enumerate() {
                    let w = (weights[k] * amount).min(1.0) * free(object, v, hid);
                    object.paint_vertex(v, target, [true, false, false, false, false], w, false);
                }
                object.composite_paint(Some(&found));
                self.smudge = Some(carried.lerp(here, 0.35));
            } else {
                self.smudge = Some(here);
            }
        } else if s.kind == BrushKind::Paint {
            // Paints the active paint layer (invert erases it) or the base, then recomposites what it touched; with
            // no layers it paints the mesh directly.
            object.mesh.ensure_paint();
            for (k, &v) in found.iter().enumerate() {
                let w = (weights[k] * amount).min(1.0) * free(object, v, hid);
                object.paint_vertex(v, s.paint, s.paint_channels, w, s.invert);
            }
            object.composite_paint(Some(&found));
        } else if s.kind == BrushKind::Mask {
            for (k, &v) in found.iter().enumerate() {
                if hid && free(object, v, hid) == 0.0 && object.mask[v as usize] < 1.0 {
                    continue;
                }
                let m = &mut object.mask[v as usize];
                *m = (*m + weights[k] * amount * sign).clamp(0.0, 1.0);
            }
        } else if matches!(s.kind, BrushKind::Smooth | BrushKind::Relax) {
            // Relax keeps only the part of the move that lies in the surface, so it evens the triangles without
            // shrinking the form; smooth also moves along the normal and wears the form down.
            let relax = s.kind == BrushKind::Relax;
            // One averaging pass spreads about one edge length, so on a dense mesh a single pass per dab cannot reach
            // the size of the brush and smoothing all but stops working. Smooth runs as many passes as spread a fifth
            // of the radius (spread grows with the square root of the passes), capped at 64; a coarse mesh keeps one.
            let passes = if relax {
                1
            } else {
                let mut sum = 0.0;
                let mut count = 0usize;
                for &v in found.iter().take(256) {
                    let p = object.mesh.position(v);
                    for &w in object.topology.neighbors_of(v) {
                        sum += (object.mesh.position(w) - p).length();
                        count += 1;
                    }
                }
                let edge = if count > 0 { sum / count as f32 } else { 0.0 };
                if edge > 0.0 { ((r * 0.2 / edge).powi(2).round() as usize).clamp(1, 64) } else { 1 }
            };
            for _ in 0..passes {
                let targets: Vec<Vec3> = found
                    .iter()
                    .map(|&v| {
                        let nb = object.topology.neighbors_of(v);
                        let p = object.mesh.position(v);
                        if nb.is_empty() {
                            return p;
                        }
                        let avg = nb.iter().map(|&w| object.mesh.position(w)).sum::<Vec3>() / nb.len() as f32;
                        if relax {
                            let n = Vec3::from_array(object.mesh.normals[v as usize]);
                            let d = avg - p;
                            p + (d - n * d.dot(n))
                        } else {
                            avg
                        }
                    })
                    .collect();
                for (k, &v) in found.iter().enumerate() {
                    let w = (weights[k] * amount).min(1.0) * free(object, v, hid);
                    let p = object.mesh.position(v);
                    object.mesh.positions[v as usize] = p.lerp(targets[k], w * 0.6).to_array();
                }
            }
        } else if s.kind == BrushKind::LayerReset {
            // The layer's delta only changes when the stroke ends, so each vertex keeps its contribution from first
            // touch and the share still left; every dab removes a falloff-weighted part of what is left.
            let at = object.layers.active.expect("checked");
            let layer_strength = object.layers.list[at].shape_scale();
            for (k, &v) in found.iter().enumerate() {
                let w = (weights[k] * amount).min(1.0) * free(object, v, hid);
                if w <= 0.0 {
                    continue;
                }
                let (contribution, left) = *self.reset_left.entry(v).or_insert_with(|| {
                    let d = object.layers.list[at].delta.get(v as usize).copied().unwrap_or([0.0; 3]);
                    (Vec3::from_array(d) * layer_strength, 1.0)
                });
                let now = left * (1.0 - w);
                self.reset_left.insert(v, (contribution, now));
                let p = object.mesh.position(v);
                object.mesh.positions[v as usize] = (p - contribution * (left - now)).to_array();
            }
        } else if s.kind == BrushKind::Polish {
            // Cuts what stands out of the dab's fitted plane, then pulls each vertex toward the average of the
            // neighbours whose normals lie within 40 degrees of its own, so creases are neither cut nor rounded.
            let cos_edge = 40f32.to_radians().cos();
            let targets: Vec<(Vec3, bool)> = found
                .iter()
                .map(|&v| {
                    let p = object.mesh.position(v);
                    let vn = Vec3::from_array(object.mesh.normals[v as usize]);
                    let (mut sum, mut total, mut crease) = (Vec3::ZERO, 0.0, false);
                    for &u in object.topology.neighbors_of(v) {
                        let k = ((vn.dot(Vec3::from_array(object.mesh.normals[u as usize])) - cos_edge) / (1.0 - cos_edge)).clamp(0.0, 1.0);
                        crease |= k == 0.0;
                        sum += object.mesh.position(u) * k;
                        total += k;
                    }
                    (if total > 1e-6 { sum / total } else { p }, crease)
                })
                .collect();
            for (k, &v) in found.iter().enumerate() {
                let w = weights[k] * amount * free(object, v, hid);
                if w <= 0.0 {
                    continue;
                }
                let p = object.mesh.position(v);
                let vn = Vec3::from_array(object.mesh.normals[v as usize]);
                let d = (p - area_c).dot(area_n);
                let similar = if targets[k].1 { 0.0 } else { ((vn.dot(area_n) - cos_edge) / 0.15).clamp(0.0, 1.0) };
                let cut = if d * sign > 0.0 { area_n * (d * (w * 0.5).min(1.0) * similar) } else { Vec3::ZERO };
                let smooth = if targets[k].1 { Vec3::ZERO } else { (targets[k].0 - p) * (w * 0.3).min(1.0) };
                let moved = self.cap_move(v, p, p - cut + smooth);
                object.mesh.positions[v as usize] = moved.to_array();
            }
        } else {
            let (plane_c, plane_n) = *self.planes[copy].get_or_insert((area_c, area_n));
            let noise_scale = if s.noise_scale > 0.0 { s.noise_scale } else { r * 0.2 };
            for (k, &v) in found.iter().enumerate() {
                let w = weights[k] * amount * free(object, v, hid);
                if w == 0.0 || (w < 0.0 && !signed) {
                    continue;
                }
                let p = object.mesh.position(v);
                let vn = Vec3::from_array(object.mesh.normals[v as usize]);
                let moved = match s.kind {
                    BrushKind::Draw | BrushKind::Stamp => p + area_n * (w * r * 0.06 * sign),
                    BrushKind::Layer => {
                        let cap = if s.height > 0.0 { s.height } else { s.radius * 0.1 };
                        let entry = self.layer_acc.entry(v).or_insert((0.0, vn));
                        let step = ((entry.0 + w * r * 0.06).min(cap) - entry.0).max(0.0);
                        entry.0 += step;
                        p + entry.1 * (step * sign)
                    }
                    BrushKind::Inflate => p + vn * (w * r * 0.06 * sign),
                    BrushKind::Clay | BrushKind::ClayStrips => {
                        let plane = area_c + area_n * (r * 0.08 * sign);
                        let d = (p - plane).dot(area_n) * sign;
                        if d < 0.0 {
                            p - area_n * (d * sign * w.min(1.0) * 0.5)
                        } else {
                            p
                        }
                    }
                    BrushKind::Flatten => p - area_n * ((p - area_c).dot(area_n) * (w * 0.5).min(1.0)),
                    BrushKind::Pinch => {
                        let to = center - p;
                        let mut tangent = to - area_n * to.dot(area_n);
                        if let Some(a) = along {
                            tangent -= a * tangent.dot(a);
                        }
                        p + tangent * (w * 0.15 * pull * sign).clamp(-1.0, 1.0)
                    }
                    BrushKind::Crease => {
                        let to = center - p;
                        let mut tangent = to - area_n * to.dot(area_n);
                        if let Some(a) = along {
                            tangent -= a * tangent.dot(a);
                        }
                        p - area_n * (w * r * 0.04 * sign) + tangent * (w * 0.2 * pull).min(1.0)
                    }
                    BrushKind::Planar => p - plane_n * ((p - plane_c).dot(plane_n) * (w * 0.5).min(1.0)),
                    BrushKind::Trim => {
                        // Only what stands above the dab's plane is cut down (inverted: only hollows are filled).
                        let d = (p - area_c).dot(area_n);
                        if d * sign > 0.0 {
                            p - area_n * (d * (w * 0.5).min(1.0))
                        } else {
                            p
                        }
                    }
                    BrushKind::Noise => p + vn * (fbm(p / noise_scale) * w * r * 0.05 * sign),
                    BrushKind::Smooth | BrushKind::Relax | BrushKind::Grab | BrushKind::Mask | BrushKind::Paint | BrushKind::SnakeHook | BrushKind::Smudge | BrushKind::Elastic | BrushKind::Polish | BrushKind::Nudge | BrushKind::LayerReset => p,
                };
                let moved = self.cap_move(v, p, moved);
                object.mesh.positions[v as usize] = moved.to_array();
            }
        }
        self.finish_dab(object, &found);
        self.found = found;
    }

    fn finish_dab(&mut self, object: &mut Object, verts: &[u32]) {
        if self.settings.kind.moves_vertices() {
            let mut ring = Vec::new();
            object.topology.ring_vertices(verts, &mut ring);
            object.topology.update_normals(&mut object.mesh, &ring);
            self.changed.extend_from_slice(&ring);
        } else {
            self.changed.extend_from_slice(verts);
        }
    }

    fn begin_grab(&mut self, object: &mut Object, anchor: Vec3, plane_normal: Vec3) {
        let s = self.settings;
        let hid = object.mesh.has_hidden();
        let kelvin = s.kind == BrushKind::Elastic;
        let reach = if kelvin { s.radius * KELVIN_REACH } else { s.radius };
        let mut copies = Vec::new();
        for (copy, m) in self.transforms.clone().into_iter().enumerate() {
            let c = m * anchor;
            let mut found = Vec::new();
            object.bvh.vertices_in_sphere(&object.mesh, c, reach, &mut found);
            let mult = self.filter(object, copy, &found, c, s.radius);
            let list = found
                .iter()
                .enumerate()
                .map(|(k, &v)| {
                    let p = object.mesh.position(v);
                    let w = if kelvin { 1.0 } else { s.falloff.weight(p.distance(c) / s.radius) } * free(object, v, hid) * mult.as_ref().map_or(1.0, |m| m[k]);
                    (v, w, p)
                })
                .collect();
            for &v in &found {
                self.remember(object, v);
            }
            copies.push((m, list));
        }
        self.grab = Some(Grab { anchor, plane_normal, kelvin, copies });
    }

    fn apply_grab(&mut self, object: &mut Object, delta: Vec3, pressure: f32) {
        let Some(grab) = self.grab.take() else { return };
        let mut touched = Vec::new();
        let amount = self.settings.strength.max(0.05) * pressure.max(0.05) * 2.0;
        for (m, list) in &grab.copies {
            let d = *m * delta;
            let centre = *m * grab.anchor;
            for &(v, w, p0) in list {
                let moved = if grab.kelvin { kelvinlet(p0 - centre, d * (w * amount).min(1.0), self.settings.radius) } else { d * (w * amount).min(1.0) };
                object.mesh.positions[v as usize] = (p0 + moved).to_array();
                touched.push(v);
            }
        }
        touched.sort_unstable();
        touched.dedup();
        self.grab = Some(grab);
        self.finish_dab(object, &touched);
        object.refit_vertices(&touched);
        self.dabs += 1;
    }

    fn texture_dab(&mut self, object: &mut Object, copy: usize, center: Vec3, pressure: f32) {
        let Some(mut tex) = object.texture.take() else { return };
        if object.has_uvs() {
            self.texture_dab_in(object, &mut tex, copy, center, pressure);
        }
        object.texture = Some(tex);
    }

    fn texture_dab_in(&mut self, object: &Object, tex: &mut Texture, copy: usize, center: Vec3, pressure: f32) {
        let s = self.settings;
        let m = &object.mesh;
        let r = self.dab_radius(pressure);
        let mut center = center;
        let mut tris = Vec::new();
        object.bvh.triangles_in_sphere(m, center, r, &mut tris);
        if tris.is_empty() {
            return;
        }
        let mut area_n = triangle_normal(m, &tris, center, r, &s);
        if s.offset != 0.0 {
            center += area_n * (s.offset * r);
            object.bvh.triangles_in_sphere(m, center, r, &mut tris);
            if tris.is_empty() {
                return;
            }
            area_n = triangle_normal(m, &tris, center, r, &s);
        }
        if copy == 0 {
            self.last_normal = area_n;
        }
        let compensation = if s.spacing_compensation { (s.spacing / 0.15).min(4.0) } else { 1.0 };
        let amount = s.strength * if s.pressure_strength { pressure } else { 1.0 } * self.jitter.1 * self.dab_mul.1 * compensation;
        let prev = self.prev_dab[copy].replace(center);
        let frame = self.alpha.clone().map(|alpha| {
            let up_ref = match (s.roll, prev) {
                (true, Some(p)) if p.distance(center) > 1e-7 => center - p,
                _ => self.transforms[copy] * Vec3::from(s.alpha_up),
            };
            let y_axis = (up_ref - area_n * up_ref.dot(area_n)).try_normalize().unwrap_or_else(|| area_n.any_orthonormal_vector());
            (alpha, y_axis.cross(area_n), y_axis)
        });
        let mut verts: Vec<u32> = tris.iter().flat_map(|&t| m.triangle(t as usize)).collect();
        verts.sort_unstable();
        verts.dedup();
        let mult = self.filter(object, copy, &verts, center, r);
        let hid = m.has_hidden();
        let freedom: Vec<f32> = verts.iter().enumerate().map(|(k, &v)| free(object, v, hid) * mult.as_ref().map_or(1.0, |x| x[k])).collect();
        let mut hits = Vec::new();
        let mut cand: Vec<(u32, f32)> = Vec::new();
        for &t in &tris {
            let Some(window) = uv_window(m, t as usize, center, r, tex.size) else { continue };
            hits.clear();
            rasterize(m, tex.size, t as usize, PAD, Some(window), &mut hits);
            let ids = m.triangle(t as usize);
            let pos = ids.map(|v| m.position(v));
            let own = ids.map(|v| freedom[verts.binary_search(&v).expect("listed")]);
            for h in &hits {
                if !h.inside && tex.interior[h.at as usize] {
                    continue;
                }
                let p = pos[0] * h.bary[0] + pos[1] * h.bary[1] + pos[2] * h.bary[2];
                let d = p.distance(center) / r;
                if d > 1.0 {
                    continue;
                }
                let mut w = s.falloff.weight(d);
                if let Some((alpha, x_axis, y_axis)) = &frame {
                    let o = p - center;
                    let (x, y) = crate::alpha::turn(o.dot(*x_axis) / r, o.dot(*y_axis) / r, s.alpha_angle + self.jitter.0 + self.tilt + s.alpha_rotation);
                    w *= alpha.stamp(x, y, s.alpha_stretch, s.alpha_tile, s.alpha_invert, s.alpha_mid).max(0.0);
                }
                w *= own[0] * h.bary[0] + own[1] * h.bary[1] + own[2] * h.bary[2];
                let w = (w * amount).min(1.0);
                if w > 0.0 {
                    cand.push((h.at, w));
                }
            }
        }
        cand.sort_unstable_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
        let target = if s.invert { fsc_core::DEFAULT_PAINT } else { s.paint };
        let mut i = 0;
        while i < cand.len() {
            let mut j = i;
            while j + 1 < cand.len() && cand[j + 1].0 == cand[i].0 {
                j += 1;
            }
            let (at, w) = cand[j];
            let cur = &mut tex.texels[at as usize];
            self.tex_before.entry(at).or_insert(*cur);
            for c in 0..PAINT_LEN {
                if s.paint_channels[paint_channel(c)] {
                    cur[c] += (target[c] - cur[c]) * w;
                }
            }
            i = j + 1;
        }
        if !cand.is_empty() {
            tex.revision += 1;
        }
    }

    pub fn drain_changed(&mut self) -> Vec<u32> {
        let mut out = std::mem::take(&mut self.changed);
        out.sort_unstable();
        out.dedup();
        out
    }

    pub fn finish(self, object: &mut Object) -> StrokeEdit {
        if self.topology_changed {
            if let Some((mesh, mask)) = self.snapshot {
                if fsc_topo::compact(&mut object.mesh, &mut object.mask) > 0 {
                    object.retopologize();
                }
                object.revision += 1;
                return StrokeEdit::Topology { mesh, mask };
            }
        }
        if !self.tex_before.is_empty() {
            let mut keys: Vec<u32> = self.tex_before.keys().copied().collect();
            keys.sort_unstable();
            let mut edit = TextureEdit::default();
            if let Some(tex) = &object.texture {
                for at in keys {
                    let (was, now) = (self.tex_before[&at], tex.texels[at as usize]);
                    if was != now {
                        edit.texels.push(at);
                        edit.before.push(was);
                        edit.after.push(now);
                    }
                }
            }
            if edit.is_empty() {
                return StrokeEdit::Nothing;
            }
            object.revision += 1;
            return StrokeEdit::Texels(edit);
        }
        let edit = self.vertex_edit(object);
        if edit.is_empty() {
            StrokeEdit::Nothing
        } else {
            StrokeEdit::Vertices(edit)
        }
    }

    fn vertex_edit(self, object: &mut Object) -> VertexEdit {
        let mut indices: Vec<u32> = self.before.keys().copied().collect();
        indices.sort_unstable();
        let mut edit = VertexEdit::default();
        let painted = object.mesh.has_paint();
        let target = self.paint_target.filter(|_| object.paint_layers.valid(object.mesh.vertex_count()));
        let mut layer = target.map(|t| (t, Vec::new(), Vec::new()));
        for &v in &indices {
            let (pos, mask, paint) = self.before[&v];
            let (now, now_mask) = (object.mesh.positions[v as usize], object.mask[v as usize]);
            let now_paint = object.mesh.paint.get(v as usize).copied().unwrap_or(paint);
            let layer_now = layer.as_ref().and_then(|_| object.paint_target(v));
            let layer_was = self.layer_before.get(&v).copied();
            if pos == now && mask == now_mask && paint == now_paint && layer_now == layer_was {
                continue;
            }
            if let (Some((_, before, after)), Some(was), Some(now)) = (layer.as_mut(), layer_was, layer_now) {
                before.push(was);
                after.push(now);
            }
            edit.indices.push(v);
            edit.before.push(pos);
            edit.after.push(now);
            edit.mask_before.push(mask);
            edit.mask_after.push(now_mask);
            if painted {
                edit.paint_before.push(paint);
                edit.paint_after.push(now_paint);
            }
        }
        if layer.as_ref().is_some_and(|(_, before, _)| before.len() == edit.indices.len()) {
            edit.layer = layer;
        }
        if !edit.is_empty() {
            object.revision += 1;
        }
        edit
    }
}

fn triangle_normal(m: &Mesh, tris: &[u32], center: Vec3, r: f32, s: &BrushSettings) -> Vec3 {
    let (mut weighted, mut plain) = (Vec3::ZERO, Vec3::ZERO);
    for &t in tris {
        let [a, b, c] = m.triangle(t as usize);
        let (pa, pb, pc) = (m.position(a), m.position(b), m.position(c));
        let n = (pb - pa).cross(pc - pa);
        weighted += n * s.falloff.weight(((pa + pb + pc) / 3.0).distance(center) / r);
        plain += n;
    }
    let n = if weighted.length_squared() > 1e-20 { weighted } else { plain };
    n.normalize_or(Vec3::Y)
}

fn uv_window(m: &Mesh, t: usize, center: Vec3, r: f32, size: usize) -> Option<(Vec2, Vec2)> {
    let [a, b, c] = m.triangle(t);
    let pa = m.position(a);
    let (v0, v1) = (m.position(b) - pa, m.position(c) - pa);
    let n = v0.cross(v1);
    let nl = n.length();
    if nl < 1e-20 {
        return None;
    }
    let h = (center - pa).dot(n) / nl;
    if h.abs() > r {
        return None;
    }
    let rp = (r * r - h * h).max(0.0).sqrt();
    let v2 = center - n / nl * h - pa;
    let (d00, d01, d11, d20, d21) = (v0.dot(v0), v0.dot(v1), v1.dot(v1), v2.dot(v0), v2.dot(v1));
    let denom = d00 * d11 - d01 * d01;
    if denom.abs() < 1e-30 {
        return None;
    }
    let (bb, bc) = ((d11 * d20 - d01 * d21) / denom, (d00 * d21 - d01 * d20) / denom);
    let (g1, g2) = (((v0 * d11 - v1 * d01) / denom).length(), ((v1 * d00 - v0 * d01) / denom).length());
    let uv = |k: usize| Vec2::from_array(m.uvs[3 * t + k]) * size as f32;
    let (qa, qb, qc) = (uv(0), uv(1), uv(2));
    let mid = qa + (qb - qa) * bb + (qc - qa) * bc;
    let half = (qb - qa).abs() * (rp * g1) + (qc - qa).abs() * (rp * g2) + Vec2::splat(PAD + 1.0);
    Some((mid - half, mid + half))
}

const KELVIN_REACH: f32 = 3.0;

/// The regularized grab Kelvinlet (de Goes and James 2017, Poisson ratio 0.4, scale `eps`) of a push `force` at offset
/// `r` from its centre, normalised so the centre moves by the full force, and eased to zero between 2 and 3 scales.
fn kelvinlet(r: Vec3, force: Vec3, eps: f32) -> Vec3 {
    let (a, b) = (1.0, 1.0 / (4.0 * (1.0 - 0.4)));
    let ra = (r.length_squared() + eps * eps).sqrt();
    let u = force * ((a - b) / ra + a * eps * eps / (2.0 * ra * ra * ra)) + r * (r.dot(force) * b / (ra * ra * ra));
    let centre = (1.5 * a - b) / eps;
    let t = (r.length() / eps - (KELVIN_REACH - 1.0)).clamp(0.0, 1.0);
    u / centre * (1.0 - t * t * (3.0 - 2.0 * t))
}

fn hash3(x: i32, y: i32, z: i32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ (z as u32).wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0x00ff_ffff) as f32 / 0x0080_0000 as f32 - 1.0
}

/// Value noise in [-1, 1]: lattice hashes blended with a smoothstep.
pub fn value_noise(p: Vec3) -> f32 {
    let f = p.floor();
    let t = p - f;
    let s = t * t * (Vec3::splat(3.0) - 2.0 * t);
    let (x, y, z) = (f.x as i32, f.y as i32, f.z as i32);
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let c = |dx, dy, dz| hash3(x + dx, y + dy, z + dz);
    let x00 = lerp(c(0, 0, 0), c(1, 0, 0), s.x);
    let x10 = lerp(c(0, 1, 0), c(1, 1, 0), s.x);
    let x01 = lerp(c(0, 0, 1), c(1, 0, 1), s.x);
    let x11 = lerp(c(0, 1, 1), c(1, 1, 1), s.x);
    lerp(lerp(x00, x10, s.y), lerp(x01, x11, s.y), s.z)
}

/// Three octaves of value noise, roughly in [-1, 1].
pub fn fbm(p: Vec3) -> f32 {
    (value_noise(p) + value_noise(p * 2.03 + Vec3::splat(17.1)) * 0.5 + value_noise(p * 4.07 + Vec3::splat(-9.3)) * 0.25) / 1.75
}

fn relax_tangential(object: &mut Object, verts: &[u32], amount: f32) {
    let targets: Vec<Vec3> = verts
        .iter()
        .map(|&v| {
            let nb = object.topology.neighbors_of(v);
            let p = object.mesh.position(v);
            if nb.len() < 3 || object.mask[v as usize] >= 1.0 {
                return p;
            }
            let avg = nb.iter().map(|&w| object.mesh.position(w)).sum::<Vec3>() / nb.len() as f32;
            let n = Vec3::from_array(object.mesh.normals[v as usize]);
            let d = avg - p;
            p + (d - n * d.dot(n)) * amount * (1.0 - object.mask[v as usize])
        })
        .collect();
    for (k, &v) in verts.iter().enumerate() {
        object.mesh.positions[v as usize] = targets[k].to_array();
    }
}
