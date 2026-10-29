//! Tone mapping and colour grading for the path render: scene light (linear Rec. 709) to display values (0 to 1,
//! sRGB-like encoded). The window's lit shading runs the same curves in its shaders (frontend gl-renderer.ts), so a
//! render and the viewport agree.

use glam::{Mat3, Vec3};

/// How scene light is brought into the display's range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// c / (c + 1): soft, keeps colours, flattens highlights.
    Reinhard,
    /// Narkowicz's fit of ACES filmic: contrasty, highlights roll to white.
    Aces,
    /// Clipped at 1: what the light is, cut off where the display ends.
    Clip,
    /// Troy Sobotka's AgX (the widely used polynomial fit): bright saturated light desaturates toward white the way
    /// film does, so hot colours do not skew in hue.
    Agx,
    /// Khronos PBR Neutral: base colours come through almost exactly; only the highlights are compressed.
    Neutral,
}

impl Tone {
    pub const NAMES: [&'static str; 5] = ["reinhard", "aces", "none", "agx", "neutral"];

    pub fn parse(name: &str) -> Option<Tone> {
        Some(match name {
            "reinhard" => Tone::Reinhard,
            "aces" => Tone::Aces,
            "none" => Tone::Clip,
            "agx" => Tone::Agx,
            "neutral" => Tone::Neutral,
            _ => return None,
        })
    }
}

/// Colour grading applied to scene light before the tone map. The default changes nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grade {
    /// A multiplier on the light (1 is as rendered; 2 is a stop brighter).
    pub exposure: f32,
    /// Contrast about mid grey (0.18): 1 is as rendered, above 1 steeper, below 1 flatter.
    pub contrast: f32,
    /// 0 is grey, 1 as rendered, 2 doubled.
    pub saturation: f32,
    /// White balance: negative is cooler (bluer), positive warmer (more orange), -1 to 1.
    pub temperature: f32,
}

impl Default for Grade {
    fn default() -> Self {
        Grade { exposure: 1.0, contrast: 1.0, saturation: 1.0, temperature: 0.0 }
    }
}

const LUMA: Vec3 = Vec3::new(0.2126, 0.7152, 0.0722);

/// The grade on linear light: white balance, exposure, saturation, then contrast in log space about mid grey.
pub fn grade(c: Vec3, g: &Grade) -> Vec3 {
    let t = g.temperature.clamp(-1.0, 1.0);
    let mut c = c * Vec3::new(1.0 + 0.18 * t, 1.0 + 0.02 * t, 1.0 - 0.22 * t) * g.exposure.max(0.0);
    let luma = c.dot(LUMA);
    c = Vec3::splat(luma).lerp(c, g.saturation.max(0.0)).max(Vec3::ZERO);
    if (g.contrast - 1.0).abs() > 1e-4 {
        let k = g.contrast.max(0.0);
        c = Vec3::new(contrast(c.x, k), contrast(c.y, k), contrast(c.z, k));
    }
    c
}

fn contrast(v: f32, k: f32) -> f32 {
    if v <= 0.0 {
        0.0
    } else {
        0.18 * (v / 0.18).powf(k)
    }
}

/// Graded, tone-mapped and display-encoded, each channel 0 to 1.
pub fn display(c: Vec3, tone: Tone, g: &Grade) -> Vec3 {
    let c = grade(c, g);
    let encode = |v: Vec3| v.clamp(Vec3::ZERO, Vec3::ONE).powf(1.0 / 2.2);
    match tone {
        Tone::Reinhard => encode(c / (c + Vec3::ONE)),
        Tone::Aces => encode((c * (2.51 * c + 0.03)) / (c * (2.43 * c + 0.59) + 0.14)),
        Tone::Clip => encode(c),
        // AgX's curve already gives display values.
        Tone::Agx => agx(c).clamp(Vec3::ZERO, Vec3::ONE),
        Tone::Neutral => encode(neutral(c)),
    }
}

fn agx(c: Vec3) -> Vec3 {
    // The inset matrix, the log2 encoding between -12.47 and +4.03 stops about mid grey, the default contrast
    // sigmoid (a sixth-degree fit) and the outset matrix back.
    let inset = Mat3::from_cols(
        Vec3::new(0.842_479_06, 0.042_328_24, 0.042_375_655),
        Vec3::new(0.078_433_6, 0.878_468_64, 0.078_433_6),
        Vec3::new(0.079_223_745, 0.079_166_13, 0.879_143),
    );
    let outset = Mat3::from_cols(
        Vec3::new(1.196_879, -0.052_896_85, -0.052_971_635),
        Vec3::new(-0.098_020_88, 1.151_903_1, -0.098_043_45),
        Vec3::new(-0.099_029_74, -0.098_961_18, 1.151_073_7),
    );
    const MIN_EV: f32 = -12.473_93;
    const MAX_EV: f32 = 4.026_069;
    let v = inset * c;
    let log = |x: f32| ((x.max(1e-10).log2()).clamp(MIN_EV, MAX_EV) - MIN_EV) / (MAX_EV - MIN_EV);
    let v = Vec3::new(log(v.x), log(v.y), log(v.z));
    let sig = |x: f32| {
        let x2 = x * x;
        let x4 = x2 * x2;
        15.5 * x4 * x2 - 40.14 * x4 * x + 31.96 * x4 - 6.868 * x2 * x + 0.4298 * x2 + 0.1191 * x - 0.00232
    };
    outset * Vec3::new(sig(v.x), sig(v.y), sig(v.z))
}

fn neutral(mut c: Vec3) -> Vec3 {
    const START: f32 = 0.8 - 0.04;
    const DESATURATION: f32 = 0.15;
    let x = c.min_element();
    let offset = if x < 0.08 { x - 6.25 * x * x } else { 0.04 };
    c -= Vec3::splat(offset);
    let peak = c.max_element();
    if peak < START {
        return c;
    }
    let d = 1.0 - START;
    let new_peak = 1.0 - d * d / (peak + d - START);
    c *= new_peak / peak;
    let g = 1.0 - 1.0 / (DESATURATION * (peak - new_peak) + 1.0);
    c.lerp(Vec3::splat(new_peak), g)
}
