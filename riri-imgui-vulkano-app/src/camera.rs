use glam::{Mat4, Quat, Vec3, Vec3A};
use imgui::{Key, Ui};
use vulkano::pipeline::graphics::viewport::Viewport;

#[derive(Debug)]
pub struct Camera {
    pub(crate) eye: Vec3A,
    pub(crate) lookat: Vec3A,
    pub(crate) up: Vec3A,
    pub(crate) pan: f32,
    pub(crate) pitch: f32,
    pub(crate) roll: f32,
    pub(crate) fovy: f32,
    pub(crate) near_clip: f32,
    pub(crate) far_clip: f32
}

impl Camera {
    pub const fn new() -> Self {
        Self {
            eye: Vec3A::new(0., 0., -5.),
            lookat: Vec3A::ZERO,
            up: Vec3A::Y,
            pan: 0.,
            pitch: 0.,
            roll: 0.,
            fovy: 60.,
            near_clip: 1.,
            far_clip: 10000.
        }
    }

    pub fn update(&mut self, ui: &Ui, delta: f32) {
        self.pan += ((ui.io().get_key_analog_value(Key::GamepadRStickLeft) * delta) - (ui.io().get_key_analog_value(Key::GamepadRStickRight) * delta)) * 2.;
        self.pitch += ((ui.io().get_key_analog_value(Key::GamepadRStickUp) * delta) - (ui.io().get_key_analog_value(Key::GamepadRStickDown) * delta)) * 2.;


        let adjustment = (ui.io().get_key_analog_value(Key::GamepadL2) * 10.) - (ui.io().get_key_analog_value(Key::GamepadR2) * 3.);

        let lh = ((ui.io().get_key_analog_value(Key::GamepadLStickRight) * delta) - (ui.io().get_key_analog_value(Key::GamepadLStickLeft) * delta)) * (5. + adjustment);
        let lv = ((ui.io().get_key_analog_value(Key::GamepadLStickUp) * delta) - (ui.io().get_key_analog_value(Key::GamepadLStickDown) * delta)) * (5. + adjustment);

        let dir = self.eye - self.lookat; // front vector
        let r = Vec3A::Y.cross(dir).normalize_or_zero(); // right unit vector
        if ui.is_key_down(Key::GamepadL1) { self.eye.y += (5. + adjustment) * delta }
        if ui.is_key_down(Key::GamepadR1) { self.eye.y -= (5. + adjustment) * delta }
        self.eye += lh * r + lv * dir.normalize_or_zero();
        self.lookat = self.eye - Vec3A::new(
            -(self.pan.sin() * self.pitch.cos()),
            self.pitch.sin(),
            -(self.pan.cos() * self.pitch.cos()),
        );
        let dir: Vec3A = self.eye - self.lookat;
        let r = Vec3A::Y.cross(dir).normalize_or_zero();
        self.up = dir.cross(r).normalize_or_zero().into();
    }

    pub fn calculate_mvp(&self, viewport: &Viewport, time_elapsed: f32) -> (Mat4, Mat4) {
        // View Projection
        let view = Mat4::look_at_rh(self.eye.into(), self.lookat.into(), self.up.into());
        let fovy_rad = self.fovy * std::f32::consts::PI / 180.;
        let mut proj = Mat4::perspective_rh(fovy_rad, viewport.extent[0] / viewport.extent[1], self.near_clip, self.far_clip);
        proj.y_axis.y *= -1.; // reverse y axis clip space for Vulkan [1, -1]
        let view_projection = proj * view;
        // Model
        let rt = time_elapsed % (std::f32::consts::PI * 2.);
        let rotation = Quat::from_euler(glam::EulerRot::XYZEx, rt, rt, 0.,);
        let model = Mat4::from_rotation_translation(rotation, Vec3::ZERO);
        (view_projection, model)
    }
}

pub(crate) const DEFAULT_CAMERA: Camera = Camera::new();