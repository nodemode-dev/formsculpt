mod bin;
mod formats;
mod gltf;
mod project;

use fsc_core::Mesh;

pub use formats::Named;
pub use gltf::{GltfObject, Resolve, Target};
pub use project::{open as open_project, save as save_project};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Obj,
    Stl,
    Ply,
    Glb,
    Gltf,
    ThreeMf,
}

impl Format {
    pub const NAMES: [&'static str; 6] = ["obj", "stl", "ply", "glb", "gltf", "3mf"];

    pub fn parse(name: &str) -> Option<Format> {
        let ext = name.rsplit('.').next().unwrap_or(name).to_ascii_lowercase();
        Some(match ext.as_str() {
            "obj" => Format::Obj,
            "stl" => Format::Stl,
            "ply" => Format::Ply,
            "glb" => Format::Glb,
            "gltf" => Format::Gltf,
            "3mf" => Format::ThreeMf,
            _ => return None,
        })
    }
}

pub fn import(format: Format, bytes: &[u8]) -> Result<Vec<Named>, String> {
    import_with(format, bytes, &|uri: &str| Err(format!("the glTF refers to an outside file ({uri}); import it from its path so the file beside it can be read, or use GLB")))
}

/// Imports a file whose glTF buffers may live in files beside it, read through `resolve`.
pub fn import_with(format: Format, bytes: &[u8], resolve: gltf::Resolve) -> Result<Vec<Named>, String> {
    let meshes = match format {
        Format::Obj => formats::read_obj(bytes)?,
        Format::Stl => formats::read_stl(bytes)?,
        Format::Ply => formats::read_ply(bytes)?,
        Format::Glb | Format::Gltf => gltf::read_gltf_with(bytes, resolve)?,
        Format::ThreeMf => formats::read_3mf(bytes)?,
    };
    for (name, mesh) in &meshes {
        mesh.validate().map_err(|e| format!("{name}: {e}"))?;
    }
    Ok(meshes)
}

pub fn export(format: Format, objects: &[GltfObject]) -> Result<Vec<u8>, String> {
    let plain: Vec<(&str, &Mesh)> = objects.iter().map(|o| (o.name, o.mesh)).collect();
    let merged = || -> Mesh {
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        let mut paint = Vec::new();
        let painted = plain.iter().any(|(_, m)| m.has_paint());
        for (_, m) in &plain {
            let base = positions.len() as u32;
            positions.extend_from_slice(&m.positions);
            indices.extend(m.indices.iter().map(|i| i + base));
            if painted {
                if m.has_paint() {
                    paint.extend_from_slice(&m.paint);
                } else {
                    paint.extend(std::iter::repeat(fsc_core::DEFAULT_PAINT).take(m.vertex_count()));
                }
            }
        }
        let mut mesh = Mesh::new(positions, indices);
        mesh.paint = paint;
        mesh
    };
    Ok(match format {
        Format::Obj => formats::write_obj(&plain),
        Format::Stl => formats::write_stl(&merged()),
        Format::Ply => formats::write_ply(&merged()),
        Format::Glb => gltf::write_glb(objects),
        Format::Gltf => return Err("a .gltf is written as two files; export it to a path (export_mesh) or as glb".into()),
        Format::ThreeMf => formats::write_3mf(&plain)?,
    })
}

/// The separate glTF form: the .gltf document pointing at `bin_name`, and the .bin's bytes.
pub fn export_gltf(objects: &[GltfObject], bin_name: &str) -> (Vec<u8>, Vec<u8>) {
    gltf::write_gltf(objects, bin_name)
}

pub fn engine_version() -> &'static str {
    fsc_core::engine_version()
}
