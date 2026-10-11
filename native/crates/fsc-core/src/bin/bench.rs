use std::time::Instant;

use fsc_core::{Bvh, Primitive};
use glam::Vec3;

fn main() {
    let level: u32 = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(8);
    let t0 = Instant::now();
    let mesh = Primitive::Sphere { radius: 1.0, subdivisions: level }.build();
    let build = t0.elapsed();
    mesh.validate().expect("valid mesh");
    let t1 = Instant::now();
    let mut normals = mesh.clone();
    normals.recompute_normals();
    let normal_time = t1.elapsed();
    let t2 = Instant::now();
    let bvh = Bvh::build(&mesh);
    let bvh_time = t2.elapsed();
    let rays = 10_000;
    let t3 = Instant::now();
    let mut hits = 0;
    for i in 0..rays {
        let a = i as f32 * 0.618_034 * std::f32::consts::TAU;
        let dir = Vec3::new(a.cos() * 0.2, (i as f32 / rays as f32) - 0.5, -1.0).normalize();
        if bvh.raycast(&mesh, Vec3::new(0.0, 0.0, 3.0), dir).is_some() {
            hits += 1;
        }
    }
    let ray_time = t3.elapsed();
    let mut found = Vec::new();
    let t4 = Instant::now();
    for i in 0..1000 {
        let p = Vec3::new((i as f32).sin(), (i as f32 * 0.7).cos(), 0.5).normalize();
        bvh.vertices_in_sphere(&mesh, p, 0.05, &mut found);
    }
    let query_time = t4.elapsed();
    println!(
        "{{\"triangles\":{},\"vertices\":{},\"build_ms\":{:.2},\"normals_ms\":{:.2},\"bvh_ms\":{:.2},\"bvh_nodes\":{},\"rays\":{},\"hits\":{},\"ray_us\":{:.3},\"sphere_queries\":1000,\"sphere_query_us\":{:.2},\"last_query_vertices\":{}}}",
        mesh.triangle_count(),
        mesh.vertex_count(),
        build.as_secs_f64() * 1e3,
        normal_time.as_secs_f64() * 1e3,
        bvh_time.as_secs_f64() * 1e3,
        bvh.nodes.len(),
        rays,
        hits,
        ray_time.as_secs_f64() * 1e6 / rays as f64,
        query_time.as_secs_f64() * 1e6 / 1000.0,
        found.len()
    );
}
