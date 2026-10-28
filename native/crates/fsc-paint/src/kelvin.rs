//! Light colour from a colour temperature in kelvin: the black-body (Planckian) locus as Kang et al. (2002) fit it in
//! CIE 1931 xy, taken to linear Rec. 709 and scaled so the brightest channel is 1. 6500 K is close to white, lower is
//! warmer (candle 1900 K, tungsten 2700 to 3200 K), higher cooler (overcast sky 7000 K, blue sky 10000 K and up). The
//! window's lights use the same fit (frontend viewport/kelvin.ts), so a light's colour matches in both.

use glam::Vec3;

pub const KELVIN_MIN: f32 = 1667.0;
pub const KELVIN_MAX: f32 = 25000.0;

/// The linear colour of light at `kelvin` (clamped to 1667 to 25000), brightest channel 1.
pub fn kelvin(kelvin: f32) -> Vec3 {
    let t = kelvin.clamp(KELVIN_MIN, KELVIN_MAX);
    let (t2, t3) = (t * t, t * t * t);
    let x = if t <= 4000.0 {
        -0.266_123_9e9 / t3 - 0.234_358_9e6 / t2 + 0.877_695_6e3 / t + 0.179_910
    } else {
        -3.025_846_9e9 / t3 + 2.107_038e6 / t2 + 0.222_634_7e3 / t + 0.240_390
    };
    let (x2, x3) = (x * x, x * x * x);
    let y = if t <= 2222.0 {
        -1.106_381_4 * x3 - 1.348_110_2 * x2 + 2.185_558_3 * x - 0.202_196_83
    } else if t <= 4000.0 {
        -0.954_947_6 * x3 - 1.374_185_9 * x2 + 2.091_37 * x - 0.167_488_67
    } else {
        3.081_758 * x3 - 5.873_387 * x2 + 3.751_13 * x - 0.370_014_83
    };
    // xy with Y = 1 to XYZ, then to linear Rec. 709 (sRGB primaries, D65).
    let (cx, cy, cz) = (x / y, 1.0, (1.0 - x - y) / y);
    let rgb = Vec3::new(
        3.240_454_2 * cx - 1.537_138_5 * cy - 0.498_531_4 * cz,
        -0.969_266 * cx + 1.876_010_8 * cy + 0.041_556 * cz,
        0.055_643_4 * cx - 0.204_025_9 * cy + 1.057_225_2 * cz,
    )
    .max(Vec3::ZERO);
    rgb / rgb.max_element().max(1e-6)
}
