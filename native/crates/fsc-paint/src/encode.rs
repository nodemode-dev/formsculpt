pub fn png_rgba(size: usize, pixels: &[[u8; 4]]) -> Result<Vec<u8>, String> {
    png_rgba_rect(size, size, pixels)
}

/// Decodes a PNG to 8-bit RGBA (gray widened, alpha 255 when absent), for projection painting.
pub fn png_rgba_decode(bytes: &[u8]) -> Result<(usize, usize, Vec<[u8; 4]>), String> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().map_err(|e| format!("not a readable PNG: {e}"))?;
    if u64::from(reader.info().width) * u64::from(reader.info().height) > 8192 * 8192 {
        return Err("the image is larger than 8192 by 8192".into());
    }
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| format!("not a readable PNG: {e}"))?;
    let (w, h) = (info.width as usize, info.height as usize);
    let channels = info.color_type.samples();
    let px = buf[..info.buffer_size()]
        .chunks_exact(channels)
        .map(|p| match channels {
            1 => [p[0], p[0], p[0], 255],
            2 => [p[0], p[0], p[0], p[1]],
            3 => [p[0], p[1], p[2], 255],
            _ => [p[0], p[1], p[2], p[3]],
        })
        .collect();
    Ok((w, h, px))
}

/// Decodes a PNG to 8-bit gray (the mean of RGB; alpha ignored), for brush alphas.
pub fn png_gray(bytes: &[u8]) -> Result<(usize, usize, Vec<u8>), String> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(bytes));
    dec.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = dec.read_info().map_err(|e| format!("not a readable PNG: {e}"))?;
    if u64::from(reader.info().width) * u64::from(reader.info().height) > 4096 * 4096 {
        return Err("the alpha image is larger than 4096 by 4096".into());
    }
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| format!("not a readable PNG: {e}"))?;
    let (w, h) = (info.width as usize, info.height as usize);
    let channels = info.color_type.samples();
    let gray = buf[..info.buffer_size()]
        .chunks_exact(channels)
        .map(|px| match channels {
            1 | 2 => px[0],
            _ => ((px[0] as u32 + px[1] as u32 + px[2] as u32) / 3) as u8,
        })
        .collect();
    Ok((w, h, gray))
}

/// An animated PNG (APNG) of equal-sized RGBA frames, looping forever at `fps`; browsers play it like a GIF but in
/// full colour with alpha.
pub fn apng_rgba(width: usize, height: usize, frames: &[Vec<[u8; 4]>], fps: u16) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, width as u32, height as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_animated(frames.len() as u32, 0).map_err(|e| e.to_string())?;
        enc.set_frame_delay(1, fps.max(1)).map_err(|e| e.to_string())?;
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        for f in frames {
            let flat: Vec<u8> = f.iter().flatten().copied().collect();
            w.write_image_data(&flat).map_err(|e| e.to_string())?;
        }
        w.finish().map_err(|e| e.to_string())?;
    }
    Ok(out)
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
