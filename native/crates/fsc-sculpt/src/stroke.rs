use std::collections::HashMap;

use fsc_core::Object;
use glam::{Mat3, Vec3};

use crate::brush::{BrushKind, BrushSettings};

#[derive(Debug, Clone, Default)]
pub struct VertexEdit {
    pub indices: Vec<u32>,
    pub before: Vec<[f32; 3]>,
    pub after: Vec<[f32; 3]>,
    pub mask_before: Vec<f32>,
    pub mask_after: Vec<f32>,
}

impl VertexEdit {
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn apply(&self, object: &mut Object, forward: bool) -> Vec<u32> {
        let (pos, mask) = if forward { (&self.after, &self.mask_after) } else { (&self.before, &self.mask_before) };
        for (k, &v) in self.indices.iter().enumerate() {
            object.mesh.positions[v as usize] = pos[k];
            object.mask[v as usize] = mask[k];
        }
        let mut ring = Vec::new();
        object.topology.ring_vertices(&self.indices, &mut ring);
        object.topology.update_normals(&mut object.mesh, &ring);
        object.bvh.refit(&object.mesh);
        object.revision += 1;
        ring
    }
}

type Captured = Vec<(u32, f32, Vec3)>;

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
    before: HashMap<u32, ([f32; 3], f32)>,
    changed: Vec<u32>,
    grab: Option<Grab>,
    found: Vec<u32>,
    pub dabs: u32,
}

impl std::fmt::Debug for Stroke {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stroke").field("settings", &self.settings).field("dabs", &self.dabs).finish()
    }
}

impl Stroke {
    pub fn new(settings: BrushSettings) -> Stroke {
        let transforms = settings.symmetry.transforms();
        Stroke { settings, transforms, last: None, lazy_point: None, before: HashMap::new(), changed: Vec::new(), grab: None, found: Vec::new(), dabs: 0 }
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
        let point = match (self.lazy_point, self.settings.lazy) {
            (Some(prev), lazy) if lazy > 0.0 => prev.lerp(point, 1.0 - lazy.clamp(0.0, 0.95)),
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
        for c in centers {
            for k in 0..self.transforms.len() {
                let m = self.transforms[k];
                self.dab(object, m * c, pressure.clamp(0.0, 1.0));
            }
            self.dabs += 1;
        }
        if self.settings.kind != BrushKind::Mask && !self.changed.is_empty() {
            object.bvh.refit(&object.mesh);
        }
    }

    fn remember(&mut self, object: &Object, v: u32) {
        self.before.entry(v).or_insert((object.mesh.positions[v as usize], object.mask[v as usize]));
    }

    fn dab(&mut self, object: &mut Object, center: Vec3, pressure: f32) {
        let s = self.settings;
        let r = s.radius;
        let mut found = std::mem::take(&mut self.found);
        object.bvh.vertices_in_sphere(&object.mesh, center, r, &mut found);
        if found.is_empty() {
            self.found = found;
            return;
        }
        let sign = if s.invert { -1.0 } else { 1.0 };
        let mut area_n = Vec3::ZERO;
        let mut area_c = Vec3::ZERO;
        let mut wsum = 0.0;
        let weights: Vec<f32> = found
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
        let area_c = if wsum > 0.0 { area_c / wsum } else { center };
        let amount = s.strength * pressure;
        for &v in &found {
            self.remember(object, v);
        }
        if s.kind == BrushKind::Mask {
            for (k, &v) in found.iter().enumerate() {
                let m = &mut object.mask[v as usize];
                *m = (*m + weights[k] * amount * sign).clamp(0.0, 1.0);
            }
        } else if s.kind == BrushKind::Smooth {
            let targets: Vec<Vec3> = found
                .iter()
                .map(|&v| {
                    let nb = object.topology.neighbors_of(v);
                    if nb.is_empty() {
                        return object.mesh.position(v);
                    }
                    nb.iter().map(|&w| object.mesh.position(w)).sum::<Vec3>() / nb.len() as f32
                })
                .collect();
            for (k, &v) in found.iter().enumerate() {
                let w = (weights[k] * amount).min(1.0) * (1.0 - object.mask[v as usize]);
                let p = object.mesh.position(v);
                object.mesh.positions[v as usize] = p.lerp(targets[k], w * 0.6).to_array();
            }
        } else {
            for (k, &v) in found.iter().enumerate() {
                let w = weights[k] * amount * (1.0 - object.mask[v as usize]);
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
                    BrushKind::Smooth | BrushKind::Grab | BrushKind::Mask => p,
                };
                object.mesh.positions[v as usize] = moved.to_array();
            }
        }
        self.finish_dab(object, &found);
        self.found = found;
    }

    fn finish_dab(&mut self, object: &mut Object, verts: &[u32]) {
        if self.settings.kind != BrushKind::Mask {
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
        let mut copies = Vec::new();
        for m in self.transforms.clone() {
            let c = m * anchor;
            let mut found = Vec::new();
            object.bvh.vertices_in_sphere(&object.mesh, c, s.radius, &mut found);
            let list = found
                .iter()
                .map(|&v| {
                    let p = object.mesh.position(v);
                    let w = s.falloff.weight(p.distance(c) / s.radius) * (1.0 - object.mask[v as usize]);
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
        object.bvh.refit(&object.mesh);
        self.dabs += 1;
    }

    pub fn drain_changed(&mut self) -> Vec<u32> {
        let mut out = std::mem::take(&mut self.changed);
        out.sort_unstable();
        out.dedup();
        out
    }

    pub fn finish(self, object: &mut Object) -> VertexEdit {
        let mut indices: Vec<u32> = self.before.keys().copied().collect();
        indices.sort_unstable();
        let mut edit = VertexEdit::default();
        for &v in &indices {
            let (pos, mask) = self.before[&v];
            let (now, now_mask) = (object.mesh.positions[v as usize], object.mask[v as usize]);
            if pos == now && mask == now_mask {
                continue;
            }
            edit.indices.push(v);
            edit.before.push(pos);
            edit.after.push(now);
            edit.mask_before.push(mask);
            edit.mask_after.push(now_mask);
        }
        if !edit.is_empty() {
            object.revision += 1;
        }
        edit
    }
}
