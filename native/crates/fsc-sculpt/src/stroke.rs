use std::collections::HashMap;

use fsc_core::{LayerPaint, Mesh, Object, Paint};
use glam::{Mat3, Vec3};

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

struct Grab {
    anchor: Vec3,
    plane_normal: Vec3,
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
    jitter: (f32, f32),
    /// The area normal under the last unmirrored dab, the plane spray scatters in.
    last_normal: Vec3,
    /// With paint layers: the paint target's values before the stroke, and which target that was.
    layer_before: HashMap<u32, LayerPaint>,
    paint_target: Option<Option<usize>>,
    pub dabs: u32,
}

pub enum StrokeEdit {
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
            jitter: (0.0, 1.0),
            last_normal: Vec3::Y,
            layer_before: HashMap::new(),
            paint_target: None,
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
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Shapes every dab by a grayscale alpha (multiplied with the falloff).
    pub fn set_alpha(&mut self, alpha: Option<std::sync::Arc<Alpha>>) {
        self.alpha = alpha;
    }

    pub fn sample_ray(&mut self, object: &mut Object, origin: Vec3, dir: Vec3, pressure: f32) -> bool {
        let dir = dir.normalize_or(Vec3::NEG_Z);
        if self.settings.kind == BrushKind::Grab {
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
        self.begin_grab(object, from, Vec3::Z);
        self.apply_grab(object, to - from, pressure);
    }

    pub fn sample_point(&mut self, object: &mut Object, point: Vec3, pressure: f32) {
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
            let c = if self.settings.spray > 0.0 && self.settings.kind != BrushKind::SnakeHook {
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
            if self.settings.kind == BrushKind::SnakeHook {
                if let Some(prev) = self.snake_prev {
                    for k in 0..self.transforms.len() {
                        let m = self.transforms[k];
                        self.snake_dab(object, m * prev, m * (c - prev));
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
        if matches!(self.settings.kind, BrushKind::Mask | BrushKind::Grab | BrushKind::Paint | BrushKind::Smudge) {
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
        for k in 0..self.transforms.len() {
            let m = self.transforms[k];
            splits += fsc_topo::refine_sphere(&mut object.mesh, &mut object.mask, m * mid, reach, max_edge, 2);
            splits += fsc_topo::collapse_sphere(&mut object.mesh, &mut object.mask, m * mid, reach, max_edge * 0.4);
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
            for _ in 0..2 {
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
    fn snake_dab(&mut self, object: &mut Object, from: Vec3, delta: Vec3) {
        let s = self.settings;
        let mut found = std::mem::take(&mut self.found);
        object.bvh.vertices_in_sphere(&object.mesh, from, s.radius, &mut found);
        // The core follows the pointer fully at any strength: a partial move would leave the pulled area behind the
        // brush, out of reach of the next dab, and the hook would stall. Strength widens the pulled area instead.
        let soft = 1.5 - s.strength.clamp(0.0, 1.0);
        let hid = object.mesh.has_hidden();
        for &v in &found {
            self.remember(object, v);
            let p = object.mesh.position(v);
            let w = s.falloff.weight(p.distance(from) / s.radius).powf(soft).min(1.0) * free(object, v, hid);
            object.mesh.positions[v as usize] = (p + delta * w).to_array();
        }
        self.finish_dab(object, &found);
        self.found = found;
    }

    fn dab(&mut self, object: &mut Object, copy: usize, center: Vec3, pressure: f32) {
        let s = self.settings;
        let r = if s.pressure_size { s.radius * pressure.max(0.1) } else { s.radius };
        let mut found = std::mem::take(&mut self.found);
        object.bvh.vertices_in_sphere(&object.mesh, center, r, &mut found);
        if found.is_empty() {
            self.found = found;
            return;
        }
        let sign = if s.invert { -1.0 } else { 1.0 };
        let hid = object.mesh.has_hidden();
        let mut area_n = Vec3::ZERO;
        let mut area_c = Vec3::ZERO;
        let mut wsum = 0.0;
        let mut weights: Vec<f32> = found
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
        let area_n = area_n.normalize_or(Vec3::Y);
        if copy == 0 {
            self.last_normal = area_n;
        }
        let area_c = if wsum > 0.0 { area_c / wsum } else { center };
        let amount = s.strength * if s.pressure_strength { pressure } else { 1.0 } * self.jitter.1;
        // The alpha lies in the plane of the area under the brush. Its top points along `alpha_up` (mirrored with the
        // copy), or with roll along the stroke's direction since the previous dab, then turns by `alpha_angle`.
        let prev = self.prev_dab[copy].replace(center);
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
                let (x, y) = crate::alpha::turn(d.dot(x_axis) / r, d.dot(y_axis) / r, s.alpha_angle + self.jitter.0);
                weights[k] *= alpha.sample(x, y);
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
                let target = [carried.x, carried.y, carried.z, 0.5, 0.0];
                for (k, &v) in found.iter().enumerate() {
                    let w = (weights[k] * amount).min(1.0) * free(object, v, hid);
                    object.paint_vertex(v, target, [true, false, false], w, false);
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
        } else {
            let (plane_c, plane_n) = *self.planes[copy].get_or_insert((area_c, area_n));
            let noise_scale = if s.noise_scale > 0.0 { s.noise_scale } else { r * 0.2 };
            for (k, &v) in found.iter().enumerate() {
                let w = weights[k] * amount * free(object, v, hid);
                if w <= 0.0 {
                    continue;
                }
                let p = object.mesh.position(v);
                let vn = Vec3::from_array(object.mesh.normals[v as usize]);
                let moved = match s.kind {
                    BrushKind::Draw => p + area_n * (w * r * 0.06 * sign),
                    BrushKind::Inflate => p + vn * (w * r * 0.06 * sign),
                    BrushKind::Clay => {
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
                        let tangent = to - area_n * to.dot(area_n);
                        p + tangent * (w * 0.15 * sign).clamp(-1.0, 1.0)
                    }
                    BrushKind::Crease => {
                        let to = center - p;
                        let tangent = to - area_n * to.dot(area_n);
                        p - area_n * (w * r * 0.04 * sign) + tangent * (w * 0.2).min(1.0)
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
                    BrushKind::Smooth | BrushKind::Relax | BrushKind::Grab | BrushKind::Mask | BrushKind::Paint | BrushKind::SnakeHook | BrushKind::Smudge => p,
                };
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
        let mut copies = Vec::new();
        for m in self.transforms.clone() {
            let c = m * anchor;
            let mut found = Vec::new();
            object.bvh.vertices_in_sphere(&object.mesh, c, s.radius, &mut found);
            let list = found
                .iter()
                .map(|&v| {
                    let p = object.mesh.position(v);
                    let w = s.falloff.weight(p.distance(c) / s.radius) * free(object, v, hid);
                    (v, w, p)
                })
                .collect();
            for &v in &found {
                self.remember(object, v);
            }
            copies.push((m, list));
        }
        self.grab = Some(Grab { anchor, plane_normal, copies });
    }

    fn apply_grab(&mut self, object: &mut Object, delta: Vec3, pressure: f32) {
        let Some(grab) = self.grab.take() else { return };
        let mut touched = Vec::new();
        let amount = self.settings.strength.max(0.05) * pressure.max(0.05) * 2.0;
        for (m, list) in &grab.copies {
            let d = *m * delta;
            for &(v, w, p0) in list {
                object.mesh.positions[v as usize] = (p0 + d * (w * amount).min(1.0)).to_array();
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
