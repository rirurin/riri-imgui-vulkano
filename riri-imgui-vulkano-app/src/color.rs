use glam::{Vec3, Vec4, Vec4Swizzles};

pub struct ColorConverter;
impl ColorConverter {
    pub fn hsv_to_rgb(hue: f32, sat: f32, val: f32) -> Vec3 {
        Self::hsv_to_rgb_vec(Vec3::new(hue, sat, val))
    }
    pub fn hsv_to_rgb_vec(c: Vec3) -> Vec3 {
        // From Metaphor Refantazio HLSL shader source (45.HLSL)
        let k = Vec4::new(1.0, 2.0 / 3.0, 1.0 / 3.0, 3.0);
        let p = ((c.x + k.xyz()).fract() * 6.0 - k.w).abs();
        c.z * k.xxx().lerp((p - k.x).clamp(Vec3::ZERO, Vec3::ONE), c.y)
    }
}