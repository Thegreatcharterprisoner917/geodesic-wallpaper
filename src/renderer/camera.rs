//! Perspective camera that slowly orbits the origin.

use glam::{Mat4, Vec3};

/// A right-hand perspective camera orbiting the world origin.
///
/// The camera position is parameterised in spherical coordinates: `angle`
/// sweeps the azimuth and `elevation` tilts the eye above the XY plane.
///
/// # Examples
///
/// ```
/// use geodesic_wallpaper::renderer::camera::Camera;
///
/// let mut cam = Camera::new(16.0 / 9.0);
/// cam.orbit(0.01);
/// let _vp = cam.view_proj();
/// ```
pub struct Camera {
    /// Current azimuth angle in radians.
    pub angle: f32,
    /// Elevation above the XY plane in radians.
    pub elevation: f32,
    /// Distance from the origin to the eye.
    pub distance: f32,
    /// Vertical field of view in radians.
    pub fov_y: f32,
    /// Viewport aspect ratio (width / height).
    pub aspect: f32,
}

impl Camera {
    /// Create a camera with default orbit parameters and the given `aspect` ratio.
    pub fn new(aspect: f32) -> Self {
        Self {
            angle: 0.0,
            elevation: 0.4,
            distance: 6.0,
            fov_y: 0.8,
            aspect,
        }
    }

    /// Advance the azimuth by `delta_angle` radians.
    pub fn orbit(&mut self, delta_angle: f32) {
        self.angle += delta_angle;
    }

    /// Compute the combined view-projection matrix for the current camera state.
    pub fn view_proj(&self) -> Mat4 {
        let eye = Vec3::new(
            self.distance * self.elevation.cos() * self.angle.cos(),
            self.distance * self.elevation.cos() * self.angle.sin(),
            self.distance * self.elevation.sin(),
        );
        let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Z);
        let proj = Mat4::perspective_rh(self.fov_y, self.aspect, 0.1, 50.0);
        proj * view
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbit_increments_angle() {
        let mut cam = Camera::new(1.0);
        cam.orbit(0.5);
        assert!((cam.angle - 0.5).abs() < 1e-6);
    }

    #[test]
    fn view_proj_is_finite() {
        let cam = Camera::new(16.0 / 9.0);
        let vp = cam.view_proj();
        for row in vp.to_cols_array_2d() {
            for v in row {
                assert!(v.is_finite(), "view_proj contains non-finite value: {v}");
            }
        }
    }
}
