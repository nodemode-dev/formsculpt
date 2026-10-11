mod bin;
mod formats;
mod gltf;
mod project;

use fsc_core::Mesh;

pub use formats::Named;
pub use gltf::{GltfObject, Target};
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
    let meshes = match format {
        Format::Obj => formats::read_obj(bytes)?,
        Format::Stl => formats::read_stl(bytes)?,
        Format::Ply => formats::read_ply(bytes)?,
        Format::Glb | Format::Gltf => gltf::read_gltf(bytes)?,
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
        for (_, m) in &plain {
            let base = positions.len() as u32;
            positions.extend_from_slice(&m.positions);
            indices.extend(m.indices.iter().map(|i| i + base));
        }
        Mesh::new(positions, indices)
    };
    Ok(match format {
        Format::Obj => formats::write_obj(&plain),
        Format::Stl => formats::write_stl(&merged()),
        Format::Ply => formats::write_ply(&merged()),
        Format::Glb => gltf::write_glb(objects),
        Format::Gltf => return Err("export glTF as glb, the single-file binary form".into()),
        Format::ThreeMf => formats::write_3mf(&plain)?,
    })
}

pub fn engine_version() -> &'static str {
    fsc_core::engine_version()
}
