pub fn png_rgba(size: usize, pixels: &[[u8; 4]]) -> Result<Vec<u8>, String> {
    png_rgba_rect(size, size, pixels)
}

pub fn png_rgba_rect(width: usize, height: usize, pixels: &[[u8; 4]]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, width as u32, height as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        let flat: Vec<u8> = pixels.iter().flatten().copied().collect();
        w.write_image_data(&flat).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

fn attr(out: &mut Vec<u8>, name: &str, ty: &str, value: &[u8]) {
    out.extend_from_slice(name.as_bytes());
    out.push(0);
    out.extend_from_slice(ty.as_bytes());
    out.push(0);
    out.extend_from_slice(&(value.len() as i32).to_le_bytes());
    out.extend_from_slice(value);
}

pub fn exr_float(size: usize, values: &[f32]) -> Vec<u8> {
    let mut out = vec![0x76, 0x2f, 0x31, 0x01, 2, 0, 0, 0];
    let mut chlist = Vec::new();
    chlist.extend_from_slice(b"Y\0");
    chlist.extend_from_slice(&2i32.to_le_bytes());
    chlist.extend_from_slice(&[0, 0, 0, 0]);
    chlist.extend_from_slice(&1i32.to_le_bytes());
    chlist.extend_from_slice(&1i32.to_le_bytes());
    chlist.push(0);
    attr(&mut out, "channels", "chlist", &chlist);
    attr(&mut out, "compression", "compression", &[0]);
    let mut window = Vec::new();
    for v in [0i32, 0, size as i32 - 1, size as i32 - 1] {
        window.extend_from_slice(&v.to_le_bytes());
    }
    attr(&mut out, "dataWindow", "box2i", &window);
    attr(&mut out, "displayWindow", "box2i", &window);
    attr(&mut out, "lineOrder", "lineOrder", &[0]);
    attr(&mut out, "pixelAspectRatio", "float", &1.0f32.to_le_bytes());
    let mut center = Vec::new();
    center.extend_from_slice(&0.0f32.to_le_bytes());
    center.extend_from_slice(&0.0f32.to_le_bytes());
    attr(&mut out, "screenWindowCenter", "v2f", &center);
    attr(&mut out, "screenWindowWidth", "float", &1.0f32.to_le_bytes());
    out.push(0);
    let table = out.len();
    let line = 8 + size * 4;
    for y in 0..size {
        out.extend_from_slice(&((table + size * 8 + y * line) as u64).to_le_bytes());
    }
    for y in 0..size {
        out.extend_from_slice(&(y as i32).to_le_bytes());
        out.extend_from_slice(&((size * 4) as i32).to_le_bytes());
        for x in 0..size {
            out.extend_from_slice(&values[y * size + x].to_le_bytes());
        }
    }
    out
}
