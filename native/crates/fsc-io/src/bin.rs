use std::collections::HashMap;

use fsc_core::Mesh;

pub fn f32s(data: &[f32]) -> Vec<u8> {
    data.iter().flat_map(|x| x.to_le_bytes()).collect()
}

pub fn u32s(data: &[u32]) -> Vec<u8> {
    data.iter().flat_map(|x| x.to_le_bytes()).collect()
}

pub fn vec3s(data: &[[f32; 3]]) -> Vec<u8> {
    data.iter().flat_map(|p| p.iter().flat_map(|c| c.to_le_bytes())).collect()
}

pub fn read_f32s(bytes: &[u8]) -> Result<Vec<f32>, String> {
    if bytes.len() % 4 != 0 {
        return Err("a float buffer has a broken length".into());
    }
    Ok(bytes.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
}

pub fn read_u32s(bytes: &[u8]) -> Result<Vec<u32>, String> {
    if bytes.len() % 4 != 0 {
        return Err("an index buffer has a broken length".into());
    }
    Ok(bytes.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
}

pub fn read_vec3s(bytes: &[u8]) -> Result<Vec<[f32; 3]>, String> {
    let flat = read_f32s(bytes)?;
    if flat.len() % 3 != 0 {
        return Err("a vector buffer has a broken length".into());
    }
    Ok(flat.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect())
}

pub fn welded(triangles: &[[f32; 3]]) -> Mesh {
    let mut index: HashMap<[u32; 3], u32> = HashMap::with_capacity(triangles.len() / 2);
    let mut positions = Vec::new();
    let mut indices = Vec::with_capacity(triangles.len());
    for p in triangles {
        let k = [p[0].to_bits(), p[1].to_bits(), p[2].to_bits()];
        let i = *index.entry(k).or_insert_with(|| {
            positions.push(*p);
            positions.len() as u32 - 1
        });
        indices.push(i);
    }
    let mut out = Vec::with_capacity(indices.len());
    for t in indices.chunks_exact(3) {
        if t[0] != t[1] && t[1] != t[2] && t[0] != t[2] {
            out.extend_from_slice(t);
        }
    }
    Mesh::new(positions, out)
}

pub fn compacted(positions: &[[f32; 3]], faces: &[u32]) -> Mesh {
    let mut remap: HashMap<u32, u32> = HashMap::new();
    let mut out_p = Vec::new();
    let mut out_i = Vec::with_capacity(faces.len());
    for &v in faces {
        let i = *remap.entry(v).or_insert_with(|| {
            out_p.push(positions[v as usize]);
            out_p.len() as u32 - 1
        });
        out_i.push(i);
    }
    Mesh::new(out_p, out_i)
}
