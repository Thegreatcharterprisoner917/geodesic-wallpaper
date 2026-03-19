//! Perspective camera that slowly orbits the origin.

use glam::{Mat4, Vec3};

/// Minimum and maximum elevation angles (radians) for drift clamping.
const ELEVATION_MIN: f32 = 0.05;
const ELEVATION_MAX: f32 = 1.4;

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
    /// Speed of elevation drift in radians per second. `0.0` disables drift.
    pub elevation_speed: f32,
    /// Current direction of elevation drift: `+1.0` or `-1.0`.
    elevation_dir: f32,
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
            elevation_speed: 0.0,
            elevation_dir: 1.0,
        }
    }

    /// Create a camera with explicit parameters drawn from configuration.
    pub fn new_with_params(
        aspect: f32,
        distance: f32,
        elevation: f32,
        fov_y: f32,
        elevation_speed: f32,
    ) -> Self {
        let elevation = elevation.clamp(ELEVATION_MIN, ELEVATION_MAX);
        Self {
            angle: 0.0,
            elevation,
            distance: distance.max(0.5),
            fov_y: fov_y.clamp(0.1, 2.5),
            aspect,
            elevation_speed: elevation_speed.abs(),
            elevation_dir: 1.0,
        }
    }

    /// Advance the azimuth by `delta_angle` radians.
    pub fn orbit(&mut self, delta_angle: f32) {
        self.angle += delta_angle;
    }

    /// Advance the elevation drift by `dt` seconds, reversing at the limits.
    ///
    /// If `elevation_speed` is `0.0` this is a no-op.
    pub fn drift_elevation(&mut self, dt: f32) {
        if self.elevation_speed == 0.0 {
            return;
        }
        self.elevation += self.elevation_dir * self.elevation_speed * dt;
        if self.elevation >= ELEVATION_MAX {
            self.elevation = ELEVATION_MAX;
            self.elevation_dir = -1.0;
        } else if self.elevation <= ELEVATION_MIN {
            self.elevation = ELEVATION_MIN;
            self.elevation_dir = 1.0;
        }
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

    #[test]
    fn new_with_params_clamps_elevation() {
        let cam = Camera::new_with_params(1.0, 6.0, -1.0, 0.8, 0.0);
        assert!(cam.elevation >= ELEVATION_MIN);
        let cam2 = Camera::new_with_params(1.0, 6.0, 10.0, 0.8, 0.0);
        assert!(cam2.elevation <= ELEVATION_MAX);
    }

    #[test]
    fn drift_elevation_reverses_at_limits() {
        let mut cam = Camera::new_with_params(1.0, 6.0, ELEVATION_MAX - 0.001, 0.8, 0.1);
        cam.elevation_dir = 1.0;
        cam.drift_elevation(1.0);
        assert_eq!(cam.elevation_dir, -1.0, "should reverse at upper limit");
        assert!(cam.elevation <= ELEVATION_MAX);
    }

    #[test]
    fn drift_elevation_noop_when_speed_zero() {
        let mut cam = Camera::new_with_params(1.0, 6.0, 0.4, 0.8, 0.0);
        cam.drift_elevation(10.0);
        assert!((cam.elevation - 0.4).abs() < 1e-6);
    }
}
