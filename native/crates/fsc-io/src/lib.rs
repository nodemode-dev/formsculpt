mod bin;
mod fbx;
mod formats;
mod gltf;
mod project;
mod usd;

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
    Fbx,
    /// USD as text (.usda; a .usd that is text reads too).
    Usda,
}

impl Format {
    pub const NAMES: [&'static str; 8] = ["obj", "stl", "ply", "glb", "gltf", "3mf", "fbx", "usda"];

    pub fn parse(name: &str) -> Option<Format> {
        let ext = name.rsplit('.').next().unwrap_or(name).to_ascii_lowercase();
        Some(match ext.as_str() {
            "obj" => Format::Obj,
            "stl" => Format::Stl,
            "ply" => Format::Ply,
            "glb" => Format::Glb,
            "gltf" => Format::Gltf,
            "3mf" => Format::ThreeMf,
            "fbx" => Format::Fbx,
            "usda" | "usd" => Format::Usda,
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
        Format::Fbx => fbx::read_fbx(bytes)?,
        Format::Usda => usd::read_usda(bytes)?,
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
                    paint.extend(std::iter::repeat_n(fsc_core::DEFAULT_PAINT, m.vertex_count()));
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
        Format::Fbx => fbx::write_fbx(objects),
        Format::Usda => usd::write_usda(objects),
    })
}

/// The separate glTF form: the .gltf document pointing at `bin_name`, and the .bin's bytes.
pub fn export_gltf(objects: &[GltfObject], bin_name: &str) -> (Vec<u8>, Vec<u8>) {
    gltf::write_gltf(objects, bin_name)
}

/// A deflated zip of named files (the baked maps download, where no other archiver is at hand, as in the browser).
pub fn zip_files(files: &[(String, &[u8])]) -> Result<Vec<u8>, String> {
    use std::io::Write;
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, data) in files {
        zip.start_file(name.as_str(), opts).map_err(|e| e.to_string())?;
        zip.write_all(data).map_err(|e| e.to_string())?;
    }
    Ok(zip.finish().map_err(|e| e.to_string())?.into_inner())
}

pub fn engine_version() -> &'static str {
    fsc_core::engine_version()
}
