use std::f32::consts::{FRAC_PI_2, PI};

use glam::Vec3;

/// radius is an absolute world radius; consumers that want a multiplier read it as one. twist is in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurvePoint {
    pub pos: Vec3,
    pub radius: f32,
    pub twist: f32,
    pub sharp: bool,
    pub round: f32,
}

impl CurvePoint {
    pub fn new(pos: Vec3) -> Self {
        Self { pos, radius: 1.0, twist: 0.0, sharp: false, round: 0.0 }
    }
}

fn follow(pts: &mut [Vec3], lens: &[f32]) {
    for i in 1..pts.len() {
        let d = (pts[i] - pts[i - 1]).try_normalize().unwrap_or(Vec3::X);
        pts[i] = pts[i - 1] + d * lens[i - 1];
    }
}

fn reach(pts: &mut [Vec3], lens: &[f32], target: Vec3, anchored: bool, iterations: usize, tolerance: f32) {
    if !anchored {
        pts[0] = target;
        follow(pts, lens);
        return;
    }
    let n = pts.len();
    let anchor = pts[n - 1];
    let to = target - anchor;
    if to.length() >= lens.iter().sum::<f32>() {
        let dir = to.try_normalize().unwrap_or(Vec3::X);
        for i in (0..n - 1).rev() {
            pts[i] = pts[i + 1] + dir * lens[i];
        }
        return;
    }
    for _ in 0..iterations {
        if pts[0].distance(target) <= tolerance {
            break;
        }
        pts[0] = target;
        follow(pts, lens);
        pts[n - 1] = anchor;
        for i in (0..n - 1).rev() {
            let d = (pts[i] - pts[i + 1]).try_normalize().unwrap_or(Vec3::X);
            pts[i] = pts[i + 1] + d * lens[i];
        }
    }
}

/// FABRIK on the control points as an open chain of fixed-length segments (a closed curve is solved as open). Returns the new positions, whether the dragged point came within tolerance of the target, and its distance from the target. With both ends pinned, the two segments beside the dragged point match their lengths to within the tolerance.
pub fn solve_ik(points: &[CurvePoint], index: usize, target: Vec3, pin_start: bool, pin_end: bool, iterations: usize, tolerance: f32) -> (Vec<Vec3>, bool, f32) {
    let pos: Vec<Vec3> = points.iter().map(|p| p.pos).collect();
    let n = pos.len();
    if n < 2 || index >= n {
        return (pos, false, f32::INFINITY);
    }
    let lens: Vec<f32> = pos.windows(2).map(|w| w[0].distance(w[1])).collect();
    let mut t = target;
    if (index == 0 && pin_start) || (index == n - 1 && pin_end) {
        t = pos[index];
    } else if pin_start && pin_end {
        let (a, b) = (pos[0], pos[n - 1]);
        let (ra, rb) = (lens[..index].iter().sum::<f32>(), lens[index..].iter().sum::<f32>());
        let inside = |p: Vec3, c: Vec3, r: f32| p.distance(c) <= r;
        if !(inside(t, a, ra) && inside(t, b, rb)) {
            let on = |c: Vec3, r: f32| c + (t - c).try_normalize().unwrap_or(Vec3::X) * r;
            let (ta, tb) = (on(a, ra), on(b, rb));
            let span = b - a;
            let d = span.length();
            if !inside(t, a, ra) && inside(ta, b, rb) {
                t = ta;
            } else if !inside(t, b, rb) && inside(tb, a, ra) {
                t = tb;
            } else if d > 1e-9 {
                let n = span / d;
                let x = (d * d + ra * ra - rb * rb) / (2.0 * d);
                let c = a + n * x;
                let rho = (ra * ra - x * x).max(0.0).sqrt();
                let v = t - c;
                let flat = (v - n * v.dot(n)).try_normalize().unwrap_or_else(|| n.any_orthonormal_vector());
                t = c + flat * rho;
            }
        }
    }
    let lower_pinned = pin_start && index > 0;
    let upper_pinned = pin_end && index < n - 1;
    let mut lo: Vec<Vec3> = pos[..=index].iter().rev().copied().collect();
    let lo_lens: Vec<f32> = lens[..index].iter().rev().copied().collect();
    let mut up: Vec<Vec3> = pos[index..].to_vec();
    let up_lens: Vec<f32> = lens[index..].to_vec();
    let mut kpos = t;
    if lower_pinned {
        reach(&mut lo, &lo_lens, t, true, iterations, tolerance);
        kpos = lo[0];
    }
    if upper_pinned {
        reach(&mut up, &up_lens, kpos, true, iterations, tolerance);
        if !lower_pinned {
            kpos = up[0];
        }
    }
    if !lower_pinned && lo.len() > 1 {
        reach(&mut lo, &lo_lens, kpos, false, iterations, tolerance);
    }
    if !upper_pinned && up.len() > 1 {
        reach(&mut up, &up_lens, kpos, false, iterations, tolerance);
    }
    let mut out = pos;
    for (i, p) in lo.iter().enumerate() {
        out[index - i] = *p;
    }
    for (j, p) in up.iter().enumerate() {
        out[index + j] = *p;
    }
    out[index] = kpos;
    let error = kpos.distance(target);
    (out, error <= tolerance, error)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interp {
    #[default]
    CatmullRom,
    BSpline,
    Linear,
}

impl Interp {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "catmull_rom" => Some(Self::CatmullRom),
            "bspline" => Some(Self::BSpline),
            "linear" => Some(Self::Linear),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CurveSample {
    pub pos: Vec3,
    pub tangent: Vec3,
    pub radius: f32,
    pub twist: f32,
    pub t: f32,
    pub corner: bool,
}

#[derive(Debug, Clone)]
pub struct Curve {
    pub points: Vec<CurvePoint>,
    pub interp: Interp,
    pub closed: bool,
}

#[derive(Debug, Clone, Copy)]
struct P {
    pos: Vec3,
    r: f32,
    tw: f32,
    corner: bool,
}

enum Seg {
    Spline(Vec<P>),
    Poly(Vec<P>),
}

fn lerp_p(a: P, b: P, t: f32) -> P {
    P { pos: a.pos.lerp(b.pos, t), r: a.r + (b.r - a.r) * t, tw: a.tw + (b.tw - a.tw) * t, corner: false }
}

fn from_point(c: &CurvePoint, corner: bool) -> P {
    P { pos: c.pos, r: c.radius, tw: c.twist, corner }
}

fn fillet(prev: Vec3, c: &CurvePoint, next: Vec3, per_span: usize) -> Option<Vec<P>> {
    let (a, b) = (prev - c.pos, next - c.pos);
    let (la, lb) = (a.length(), b.length());
    if la < 1e-9 || lb < 1e-9 {
        return None;
    }
    let (ua, ub) = (a / la, b / lb);
    let theta = ua.dot(ub).clamp(-1.0, 1.0).acos();
    if !(0.01..=PI - 0.01).contains(&theta) {
        return None;
    }
    let half = theta * 0.5;
    let d = (c.round / half.tan()).min(0.5 * la).min(0.5 * lb);
    let rr = d * half.tan();
    let bis = (ua + ub).try_normalize()?;
    let center = c.pos + bis * (rr / half.sin());
    let (v1, v2) = (c.pos + ua * d - center, c.pos + ub * d - center);
    let phi = PI - theta;
    let sin_phi = phi.sin();
    if sin_phi.abs() < 1e-6 {
        return None;
    }
    let n = ((phi / FRAC_PI_2) * per_span.max(2) as f32).ceil().max(2.0) as usize;
    let arc = (0..=n)
        .map(|k| {
            let s = k as f32 / n as f32;
            let v = (v1 * (phi * (1.0 - s)).sin() + v2 * (phi * s).sin()) / sin_phi;
            P { pos: center + v, r: c.radius, tw: c.twist, corner: false }
        })
        .collect();
    Some(arc)
}

fn clamp_between(v: f32, a: f32, b: f32) -> f32 {
    v.clamp(a.min(b), a.max(b))
}

fn scalar(interp: Interp, s: [f32; 4], u: f32) -> f32 {
    match interp {
        Interp::Linear => s[1] + (s[2] - s[1]) * u,
        Interp::BSpline => {
            let (u2, u3) = (u * u, u * u * u);
            ((1.0 - u).powi(3) * s[0] + (3.0 * u3 - 6.0 * u2 + 4.0) * s[1] + (-3.0 * u3 + 3.0 * u2 + 3.0 * u + 1.0) * s[2] + u3 * s[3]) / 6.0
        }
        Interp::CatmullRom => {
            let (u2, u3) = (u * u, u * u * u);
            let v = 0.5 * (2.0 * s[1] + (-s[0] + s[2]) * u + (2.0 * s[0] - 5.0 * s[1] + 4.0 * s[2] - s[3]) * u2 + (-s[0] + 3.0 * s[1] - 3.0 * s[2] + s[3]) * u3);
            clamp_between(v, s[1], s[2])
        }
    }
}

fn centripetal(p: [Vec3; 4], u: f32) -> Vec3 {
    let knot = |a: Vec3, b: Vec3| a.distance(b).sqrt().max(1e-4);
    let t0 = 0.0;
    let t1 = t0 + knot(p[0], p[1]);
    let t2 = t1 + knot(p[1], p[2]);
    let t3 = t2 + knot(p[2], p[3]);
    let t = t1 + (t2 - t1) * u;
    let a1 = p[0] * ((t1 - t) / (t1 - t0)) + p[1] * ((t - t0) / (t1 - t0));
    let a2 = p[1] * ((t2 - t) / (t2 - t1)) + p[2] * ((t - t1) / (t2 - t1));
    let a3 = p[2] * ((t3 - t) / (t3 - t2)) + p[3] * ((t - t2) / (t3 - t2));
    let b1 = a1 * ((t2 - t) / (t2 - t0)) + a2 * ((t - t0) / (t2 - t0));
    let b2 = a2 * ((t3 - t) / (t3 - t1)) + a3 * ((t - t1) / (t3 - t1));
    b1 * ((t2 - t) / (t2 - t1)) + b2 * ((t - t1) / (t2 - t1))
}

fn position(interp: Interp, p: [Vec3; 4], u: f32) -> Vec3 {
    match interp {
        Interp::Linear => p[1].lerp(p[2], u),
        Interp::BSpline => {
            let (u2, u3) = (u * u, u * u * u);
            (p[0] * (1.0 - u).powi(3) + p[1] * (3.0 * u3 - 6.0 * u2 + 4.0) + p[2] * (-3.0 * u3 + 3.0 * u2 + 3.0 * u + 1.0) + p[3] * u3) / 6.0
        }
        Interp::CatmullRom => centripetal(p, u),
    }
}

fn spline(interp: Interp, k: &[P], periodic: bool, per_span: usize, out: &mut Vec<P>) {
    let m = k.len();
    if m < 2 {
        out.extend_from_slice(k);
        return;
    }
    if interp == Interp::Linear || (m == 2 && !periodic) {
        out.extend_from_slice(k);
        return;
    }
    let get = |i: isize| -> P {
        if periodic {
            return k[i.rem_euclid(m as isize) as usize];
        }
        if i < 0 {
            let (a, b) = (k[0], k[1]);
            return P { pos: a.pos * 2.0 - b.pos, r: 2.0 * a.r - b.r, tw: 2.0 * a.tw - b.tw, corner: false };
        }
        if i >= m as isize {
            let (a, b) = (k[m - 1], k[m - 2]);
            return P { pos: a.pos * 2.0 - b.pos, r: 2.0 * a.r - b.r, tw: 2.0 * a.tw - b.tw, corner: false };
        }
        k[i as usize]
    };
    let spans = if periodic { m } else { m - 1 };
    for s in 0..spans as isize {
        let q = [get(s - 1), get(s), get(s + 1), get(s + 2)];
        let pos = [q[0].pos, q[1].pos, q[2].pos, q[3].pos];
        let rs = [q[0].r, q[1].r, q[2].r, q[3].r];
        let ts = [q[0].tw, q[1].tw, q[2].tw, q[3].tw];
        for step in 0..per_span {
            let u = step as f32 / per_span as f32;
            if step == 0 {
                out.push(q[1]);
                continue;
            }
            out.push(P { pos: position(interp, pos, u), r: scalar(interp, rs, u).max(1e-6), tw: scalar(interp, ts, u), corner: false });
        }
    }
    if !periodic {
        out.push(k[m - 1]);
    }
}

fn append(out: &mut Vec<P>, run: Vec<P>) {
    for q in run {
        match out.last_mut() {
            Some(last) if last.pos.distance(q.pos) < 1e-7 => last.corner |= q.corner,
            _ => out.push(q),
        }
    }
}

fn polyline_length(p: &[P], closed: bool) -> f32 {
    let mut total: f32 = p.windows(2).map(|w| w[0].pos.distance(w[1].pos)).sum();
    if closed && p.len() > 2 {
        total += p[p.len() - 1].pos.distance(p[0].pos);
    }
    total
}

fn finish(p: &[P], closed: bool) -> Vec<CurveSample> {
    let n = p.len();
    let total = polyline_length(p, closed).max(1e-12);
    let mut walked = 0.0;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        if i > 0 {
            walked += p[i - 1].pos.distance(p[i].pos);
        }
        let prev = if i > 0 { Some(p[i - 1].pos) } else if closed && n > 2 { Some(p[n - 1].pos) } else { None };
        let next = if i + 1 < n { Some(p[i + 1].pos) } else if closed && n > 2 { Some(p[0].pos) } else { None };
        let a = prev.map(|q| (p[i].pos - q).normalize_or_zero()).unwrap_or(Vec3::ZERO);
        let b = next.map(|q| (q - p[i].pos).normalize_or_zero()).unwrap_or(Vec3::ZERO);
        let fallback = (next.unwrap_or(p[i].pos) - prev.unwrap_or(p[i].pos)).normalize_or(Vec3::Y);
        let tangent = (a + b).try_normalize().unwrap_or(fallback);
        out.push(CurveSample { pos: p[i].pos, tangent, radius: p[i].r, twist: p[i].tw, t: walked / total, corner: p[i].corner });
    }
    out
}

impl Curve {
    pub fn new(points: Vec<CurvePoint>, interp: Interp, closed: bool) -> Self {
        Self { points, interp, closed }
    }

    fn cleaned(&self) -> (Vec<CurvePoint>, bool) {
        let mut pts: Vec<CurvePoint> = Vec::with_capacity(self.points.len());
        for c in &self.points {
            if pts.last().is_none_or(|q| q.pos.distance(c.pos) > 1e-6) {
                pts.push(*c);
            }
        }
        if self.closed && pts.len() > 1 && pts[0].pos.distance(pts[pts.len() - 1].pos) <= 1e-6 {
            pts.pop();
        }
        let closed = self.closed && pts.len() >= 3;
        (pts, closed)
    }

    pub fn is_closed(&self) -> bool {
        self.cleaned().1
    }

    fn plan(&self, per_span: usize) -> (Vec<Seg>, bool, bool) {
        let (pts, closed) = self.cleaned();
        let n = pts.len();
        let is_break = |i: usize| pts[i].sharp && (closed || (i > 0 && i + 1 < n));
        let arc_of = |i: usize| -> Option<Vec<P>> {
            let c = &pts[i];
            if c.round <= 0.0 {
                return None;
            }
            let (prev, next) = if closed {
                (pts[(i + n - 1) % n].pos, pts[(i + 1) % n].pos)
            } else {
                (pts[i - 1].pos, pts[i + 1].pos)
            };
            fillet(prev, c, next, per_span)
        };
        let mut segs: Vec<Seg> = Vec::new();
        let mut cur: Vec<P> = Vec::new();
        let step = |i: usize,segs: &mut Vec<Seg>, cur: &mut Vec<P>| {
            if !is_break(i) {
                cur.push(from_point(&pts[i], false));
                return;
            }
            match arc_of(i) {
                Some(arc) => {
                    cur.push(arc[0]);
                    segs.push(Seg::Spline(std::mem::take(cur)));
                    *cur = vec![arc[arc.len() - 1]];
                    segs.push(Seg::Poly(arc));
                }
                None => {
                    let q = from_point(&pts[i], true);
                    cur.push(q);
                    segs.push(Seg::Spline(std::mem::take(cur)));
                    *cur = vec![q];
                }
            }
        };
        if !closed {
            for i in 0..n {
                step(i, &mut segs, &mut cur);
            }
            if cur.len() >= 2 {
                segs.push(Seg::Spline(cur));
            }
            return (segs, false, false);
        }
        let Some(b0) = (0..n).find(|&i| pts[i].sharp) else {
            return (vec![Seg::Spline(pts.iter().map(|c| from_point(c, false)).collect())], true, true);
        };
        let arc0 = arc_of(b0);
        match &arc0 {
            Some(a) => {
                segs.push(Seg::Poly(a.clone()));
                cur = vec![a[a.len() - 1]];
            }
            None => cur = vec![from_point(&pts[b0], true)],
        }
        for k in 1..n {
            step((b0 + k) % n, &mut segs, &mut cur);
        }
        cur.push(match &arc0 {
            Some(a) => a[0],
            None => from_point(&pts[b0], true),
        });
        segs.push(Seg::Spline(cur));
        (segs, true, false)
    }

    fn dense(&self, per_span: usize) -> (Vec<P>, bool) {
        let per_span = per_span.clamp(1, 256);
        let (segs, closed, periodic) = self.plan(per_span);
        let mut out: Vec<P> = Vec::new();
        for seg in segs {
            match seg {
                Seg::Spline(k) => {
                    let mut run = Vec::new();
                    spline(self.interp, &k, periodic, per_span, &mut run);
                    append(&mut out, run);
                }
                Seg::Poly(run) => append(&mut out, run),
            }
        }
        if closed && out.len() > 1 && out[0].pos.distance(out[out.len() - 1].pos) < 1e-7 {
            let last = out.pop().expect("checked");
            out[0].corner |= last.corner;
        }
        (out, closed)
    }

    pub fn sample(&self, per_span: usize) -> Vec<CurveSample> {
        let (dense, closed) = self.dense(per_span);
        finish(&dense, closed)
    }

    pub fn length(&self) -> f32 {
        let (dense, closed) = self.dense(16);
        polyline_length(&dense, closed)
    }

    pub fn sample_spacing(&self, step: f32) -> Vec<CurveSample> {
        let (mut dense, closed) = self.dense(16);
        if dense.len() < 2 {
            return finish(&dense, closed);
        }
        if closed {
            let first = dense[0];
            dense.push(P { corner: false, ..first });
        }
        let total = polyline_length(&dense, false);
        let step = step.max(total / 100_000.0).max(1e-9);
        let last = dense.len() - 1;
        let mut stops = vec![0usize];
        stops.extend((1..last).filter(|&i| dense[i].corner));
        stops.push(last);
        let mut out: Vec<P> = Vec::new();
        for w in stops.windows(2) {
            let (a, b) = (w[0], w[1]);
            let len: f32 = (a..b).map(|i| dense[i].pos.distance(dense[i + 1].pos)).sum();
            let count = ((len / step).round() as usize).max(1);
            let mut seg = a;
            let mut before = 0.0;
            for s in 0..count {
                let want = len * s as f32 / count as f32;
                while seg + 1 < b && before + dense[seg].pos.distance(dense[seg + 1].pos) < want {
                    before += dense[seg].pos.distance(dense[seg + 1].pos);
                    seg += 1;
                }
                let l = dense[seg].pos.distance(dense[seg + 1].pos).max(1e-12);
                let mut q = lerp_p(dense[seg], dense[seg + 1], ((want - before) / l).clamp(0.0, 1.0));
                q.corner = s == 0 && dense[a].corner;
                out.push(q);
            }
        }
        out.push(dense[last]);
        if closed {
            out.pop();
        }
        finish(&out, closed)
    }
}
