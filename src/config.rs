//! Runtime configuration loaded from `config.toml` with hot-reload support.
//!
//! All fields have serde defaults so the application starts with sensible
//! values even when the config file is absent or partially specified.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, RwLock};

/// Runtime configuration for the geodesic wallpaper.
///
/// Loaded from `config.toml` on startup and re-loaded whenever the file
/// changes on disk (hot-reload). Missing fields fall back to their defaults.
///
/// # Examples
///
/// ```
/// use geodesic_wallpaper::config::Config;
///
/// let cfg = Config::default();
/// assert_eq!(cfg.surface, "torus");
/// assert_eq!(cfg.num_geodesics, 30);
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct Config {
    /// Surface to render: `"torus"`, `"sphere"`, `"saddle"`, `"enneper"`, or `"catenoid"`.
    ///
    /// Any unrecognised value falls back to `"torus"`.
    #[serde(default = "default_surface")]
    pub surface: String,

    /// Number of simultaneous geodesic curves.
    ///
    /// Must be > 0. Default: `30`.
    #[serde(default = "default_num_geodesics")]
    pub num_geodesics: usize,

    /// Number of frames a trail persists before fading out.
    ///
    /// Must be > 0. Default: `300`.
    #[serde(default = "default_trail_length")]
    pub trail_length: usize,

    /// Camera orbit speed in radians per second.
    ///
    /// Default: `0.001047` (approximately one revolution every 100 minutes).
    #[serde(default = "default_rotation_speed")]
    pub rotation_speed: f32,

    /// Trail colour palette as CSS hex strings (e.g. `"#4488FF"`).
    ///
    /// Geodesics cycle through this list in `"cycle"` mode, or colours are
    /// assigned randomly in `"random"` mode. At least one colour is required;
    /// the default palette contains five entries.
    #[serde(default = "default_color_palette")]
    pub color_palette: Vec<String>,

    /// Torus major radius: distance from the torus center to the tube center.
    ///
    /// Must be > `torus_r`. Default: `2.0`.
    #[serde(default = "default_torus_r_big")]
    #[allow(non_snake_case)]
    pub torus_R: f32,

    /// Torus minor radius: tube radius.
    ///
    /// Must be > 0 and < `torus_R`. Default: `0.7`.
    #[serde(default = "default_torus_r_small")]
    pub torus_r: f32,

    /// RK4 integration timestep per frame in seconds.
    ///
    /// Default: `0.016`.
    #[serde(default = "default_time_step")]
    pub time_step: f32,

    // ── Camera ─────────────────────────────────────────────────────────────

    /// Distance from the origin to the camera eye.
    ///
    /// Default: `6.0`.
    #[serde(default = "default_camera_distance")]
    pub camera_distance: f32,

    /// Camera elevation above the XY plane in radians.
    ///
    /// Default: `0.4`.
    #[serde(default = "default_camera_elevation")]
    pub camera_elevation: f32,

    /// Vertical field-of-view in radians.
    ///
    /// Default: `0.8`.
    #[serde(default = "default_camera_fov")]
    pub camera_fov: f32,

    /// Speed at which the camera elevation drifts, in radians per second.
    ///
    /// Set to `0.0` to disable elevation drift. Elevation is clamped to
    /// `[0.05, 1.4]` and reverses at the limits. Default: `0.0`.
    #[serde(default = "default_camera_elevation_speed")]
    pub camera_elevation_speed: f32,

    // ── Rendering ──────────────────────────────────────────────────────────

    /// Whether to render the surface wireframe mesh.
    ///
    /// Default: `true`.
    #[serde(default = "default_show_wireframe")]
    pub show_wireframe: bool,

    /// Maximum number of trail vertices in the GPU buffer.
    ///
    /// Larger values consume more VRAM. Default: `100000`.
    #[serde(default = "default_max_trail_verts")]
    pub max_trail_verts: usize,

    /// Exponent for the trail alpha fade curve.
    ///
    /// `1.0` = linear, `2.0` = quadratic (default), `3.0` = cubic.
    /// Higher values make trails fade more sharply toward the tail.
    #[serde(default = "default_trail_fade_power")]
    pub trail_fade_power: f32,

    /// How to assign colours to geodesics.
    ///
    /// `"cycle"` (default) cycles through `color_palette` in order.
    /// `"random"` picks a random palette colour for each new geodesic.
    #[serde(default = "default_color_mode")]
    pub color_mode: String,

    /// Target frames per second for the render loop.
    ///
    /// Default: `30`.
    #[serde(default = "default_target_fps")]
    pub target_fps: u32,

    // ── UX ─────────────────────────────────────────────────────────────────

    /// Log FPS and surface info periodically to the console.
    ///
    /// Default: `false`.
    #[serde(default = "default_show_hud")]
    pub show_hud: bool,

    /// Show an epilepsy/photosensitivity warning on first launch.
    ///
    /// After displaying the warning once the application continues. Set to
    /// `false` to suppress. Default: `true`.
    #[serde(default = "default_epilepsy_warning")]
    pub epilepsy_warning: bool,

    /// Whether to span the wallpaper across all monitors (virtual screen).
    ///
    /// `false` (default) uses the primary monitor only.
    #[serde(default = "default_multi_monitor")]
    pub multi_monitor: bool,
}

fn default_surface() -> String {
    "torus".into()
}
fn default_num_geodesics() -> usize {
    30
}
fn default_trail_length() -> usize {
    300
}
fn default_rotation_speed() -> f32 {
    0.001047
}
fn default_color_palette() -> Vec<String> {
    vec![
        "#4488FF".into(),
        "#88DDFF".into(),
        "#FFD700".into(),
        "#88FF88".into(),
        "#FF88CC".into(),
    ]
}
fn default_torus_r_big() -> f32 {
    2.0
}
fn default_torus_r_small() -> f32 {
    0.7
}
fn default_time_step() -> f32 {
    0.016
}
fn default_camera_distance() -> f32 {
    6.0
}
fn default_camera_elevation() -> f32 {
    0.4
}
fn default_camera_fov() -> f32 {
    0.8
}
fn default_camera_elevation_speed() -> f32 {
    0.0
}
fn default_show_wireframe() -> bool {
    true
}
fn default_max_trail_verts() -> usize {
    100_000
}
fn default_trail_fade_power() -> f32 {
    2.0
}
fn default_color_mode() -> String {
    "cycle".into()
}
fn default_target_fps() -> u32 {
    30
}
fn default_show_hud() -> bool {
    false
}
fn default_epilepsy_warning() -> bool {
    true
}
fn default_multi_monitor() -> bool {
    false
}

impl Default for Config {
    fn default() -> Self {
        Config {
            surface: default_surface(),
            num_geodesics: default_num_geodesics(),
            trail_length: default_trail_length(),
            rotation_speed: default_rotation_speed(),
            color_palette: default_color_palette(),
            torus_R: default_torus_r_big(),
            torus_r: default_torus_r_small(),
            time_step: default_time_step(),
            camera_distance: default_camera_distance(),
            camera_elevation: default_camera_elevation(),
            camera_fov: default_camera_fov(),
            camera_elevation_speed: default_camera_elevation_speed(),
            show_wireframe: default_show_wireframe(),
            max_trail_verts: default_max_trail_verts(),
            trail_fade_power: default_trail_fade_power(),
            color_mode: default_color_mode(),
            target_fps: default_target_fps(),
            show_hud: default_show_hud(),
            epilepsy_warning: default_epilepsy_warning(),
            multi_monitor: default_multi_monitor(),
        }
    }
}

impl Config {
    /// Load a [`Config`] from a TOML file at `path`.
    ///
    /// If the file cannot be read, the default config is returned silently
    /// (the file is optional). If the file exists but cannot be parsed, a
    /// warning is emitted via `tracing` and the error message is printed to
    /// `stderr` so users can diagnose `config.toml` syntax issues.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use geodesic_wallpaper::config::Config;
    /// use std::path::Path;
    ///
    /// let cfg = Config::load(Path::new("config.toml"));
    /// println!("Surface: {}", cfg.surface);
    /// ```
    pub fn load(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(s) => match toml::from_str::<Config>(&s) {
                Ok(cfg) => {
                    let warnings = cfg.validate();
                    for w in &warnings {
                        tracing::warn!("Config validation: {w}");
                        eprintln!("[geodesic-wallpaper] config warning: {w}");
                    }
                    cfg
                }
                Err(e) => {
                    let msg = format!("Config parse error in '{}': {e}", path.display());
                    tracing::warn!("{msg}");
                    eprintln!("[geodesic-wallpaper] {msg}");
                    eprintln!("[geodesic-wallpaper] using default configuration");
                    Config::default()
                }
            },
            Err(_) => Config::default(),
        }
    }

    /// Validate configuration values and return a list of human-readable warnings.
    ///
    /// This does not modify the config; callers should log or display the
    /// returned messages. An empty `Vec` means the config is fully valid.
    ///
    /// # Examples
    ///
    /// ```
    /// use geodesic_wallpaper::config::Config;
    ///
    /// let cfg = Config::default();
    /// assert!(cfg.validate().is_empty());
    /// ```
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if self.num_geodesics == 0 {
            warnings.push(
                "num_geodesics is 0 — no geodesics will be rendered; set to at least 1".into(),
            );
        }
        if self.trail_length == 0 {
            warnings.push(
                "trail_length is 0 — trails will be invisible; set to at least 1".into(),
            );
        }
        if self.torus_R <= 0.0 {
            warnings.push(format!(
                "torus_R = {} is not positive — torus will degenerate",
                self.torus_R
            ));
        }
        if self.torus_r <= 0.0 {
            warnings.push(format!(
                "torus_r = {} is not positive — torus will degenerate",
                self.torus_r
            ));
        }
        if self.torus_R > 0.0 && self.torus_r >= self.torus_R {
            warnings.push(format!(
                "torus_r ({}) >= torus_R ({}) — torus will self-intersect",
                self.torus_r, self.torus_R
            ));
        }
        if self.time_step <= 0.0 {
            warnings.push(format!(
                "time_step = {} is not positive — simulation will not advance",
                self.time_step
            ));
        }
        if self.color_palette.is_empty() {
            warnings.push(
                "color_palette is empty — a fallback grey colour will be used".into(),
            );
        }
        if self.max_trail_verts < 100 {
            warnings.push(format!(
                "max_trail_verts = {} is very small — trails may be clipped",
                self.max_trail_verts
            ));
        }
        if self.target_fps == 0 {
            warnings.push("target_fps is 0 — defaulting to 30 fps".into());
        }
        if self.trail_fade_power <= 0.0 {
            warnings.push(format!(
                "trail_fade_power = {} must be positive; defaulting to 2.0",
                self.trail_fade_power
            ));
        }
        if !["cycle", "random"].contains(&self.color_mode.as_str()) {
            warnings.push(format!(
                "color_mode '{}' is unrecognised — use 'cycle' or 'random'; defaulting to 'cycle'",
                self.color_mode
            ));
        }
        let known_surfaces = ["torus", "sphere", "saddle", "enneper", "catenoid"];
        if !known_surfaces.contains(&self.surface.as_str()) {
            warnings.push(format!(
                "surface '{}' is unrecognised — known values: {}; defaulting to 'torus'",
                self.surface,
                known_surfaces.join(", ")
            ));
        }

        warnings
    }

    /// Parse a CSS hex colour string into a linear `[r, g, b, 1.0]` array.
    ///
    /// Accepts strings with or without a leading `#`. Individual channel
    /// parse failures fall back to `128` (≈ 0.502).
    ///
    /// # Examples
    ///
    /// ```
    /// use geodesic_wallpaper::config::Config;
    ///
    /// let color = Config::parse_color("#FF8800");
    /// assert!((color[0] - 1.0).abs() < 0.01);
    /// assert_eq!(color[3], 1.0);
    /// ```
    pub fn parse_color(hex: &str) -> [f32; 4] {
        let h = hex.trim_start_matches('#');
        let r = u8::from_str_radix(h.get(0..2).unwrap_or("80"), 16).unwrap_or(128) as f32 / 255.0;
        let g = u8::from_str_radix(h.get(2..4).unwrap_or("80"), 16).unwrap_or(128) as f32 / 255.0;
        let b = u8::from_str_radix(h.get(4..6).unwrap_or("80"), 16).unwrap_or(128) as f32 / 255.0;
        [r, g, b, 1.0]
    }

    /// Return the effective target FPS, clamping zero to 30.
    pub fn effective_target_fps(&self) -> u32 {
        if self.target_fps == 0 { 30 } else { self.target_fps }
    }

    /// Return the effective trail fade power, clamping non-positive values to 2.0.
    pub fn effective_fade_power(&self) -> f32 {
        if self.trail_fade_power <= 0.0 { 2.0 } else { self.trail_fade_power }
    }
}

/// Thread-safe handle to a [`Config`] that can be updated from a watcher thread.
pub type SharedConfig = Arc<RwLock<Config>>;

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn default_values_are_correct() {
        let cfg = Config::default();
        assert_eq!(cfg.surface, "torus");
        assert_eq!(cfg.num_geodesics, 30);
        assert_eq!(cfg.trail_length, 300);
        assert!((cfg.rotation_speed - 0.001047).abs() < 1e-6);
        assert!((cfg.torus_R - 2.0).abs() < 1e-6);
        assert!((cfg.torus_r - 0.7).abs() < 1e-6);
        assert!((cfg.time_step - 0.016).abs() < 1e-6);
        assert_eq!(cfg.color_palette.len(), 5);
        assert!((cfg.camera_distance - 6.0).abs() < 1e-6);
        assert!((cfg.camera_elevation - 0.4).abs() < 1e-6);
        assert!((cfg.camera_fov - 0.8).abs() < 1e-6);
        assert_eq!(cfg.show_wireframe, true);
        assert_eq!(cfg.max_trail_verts, 100_000);
        assert!((cfg.trail_fade_power - 2.0).abs() < 1e-6);
        assert_eq!(cfg.color_mode, "cycle");
        assert_eq!(cfg.target_fps, 30);
    }

    #[test]
    fn toml_parse_full_config() {
        let toml = r##"
surface = "sphere"
num_geodesics = 10
trail_length = 100
rotation_speed = 0.005
color_palette = ["#FF0000"]
torus_R = 3.0
torus_r = 1.0
time_step = 0.008
camera_distance = 8.0
camera_elevation = 0.6
camera_fov = 1.0
show_wireframe = false
max_trail_verts = 50000
trail_fade_power = 3.0
color_mode = "random"
target_fps = 60
"##;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(toml.as_bytes()).unwrap();
        let cfg = Config::load(f.path());
        assert_eq!(cfg.surface, "sphere");
        assert_eq!(cfg.num_geodesics, 10);
        assert_eq!(cfg.trail_length, 100);
        assert!((cfg.rotation_speed - 0.005).abs() < 1e-6);
        assert!((cfg.torus_R - 3.0).abs() < 1e-6);
        assert!((cfg.torus_r - 1.0).abs() < 1e-6);
        assert!((cfg.time_step - 0.008).abs() < 1e-6);
        assert!((cfg.camera_distance - 8.0).abs() < 1e-6);
        assert!((cfg.camera_elevation - 0.6).abs() < 1e-6);
        assert!((cfg.camera_fov - 1.0).abs() < 1e-6);
        assert_eq!(cfg.show_wireframe, false);
        assert_eq!(cfg.max_trail_verts, 50000);
        assert!((cfg.trail_fade_power - 3.0).abs() < 1e-6);
        assert_eq!(cfg.color_mode, "random");
        assert_eq!(cfg.target_fps, 60);
        assert_eq!(cfg.color_palette, vec!["#FF0000"]);
    }

    #[test]
    fn partial_config_falls_back_to_defaults() {
        let toml = r#"surface = "saddle""#;
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(toml.as_bytes()).unwrap();
        let cfg = Config::load(f.path());
        assert_eq!(cfg.surface, "saddle");
        assert_eq!(cfg.num_geodesics, 30);
        assert_eq!(cfg.trail_length, 300);
        assert_eq!(cfg.show_wireframe, true);
    }

    #[test]
    fn invalid_toml_returns_defaults() {
        let toml = b"this is not valid toml :::";
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(toml).unwrap();
        let cfg = Config::load(f.path());
        assert_eq!(cfg.surface, "torus");
        assert_eq!(cfg.num_geodesics, 30);
    }

    #[test]
    fn missing_file_returns_defaults() {
        let cfg = Config::load(std::path::Path::new("/nonexistent/path/config.toml"));
        assert_eq!(cfg.surface, "torus");
    }

    #[test]
    fn parse_color_full_hex() {
        let c = Config::parse_color("#FF8800");
        assert!((c[0] - 1.0).abs() < 0.01);
        assert!((c[1] - 0.533).abs() < 0.01);
        assert!((c[2] - 0.0).abs() < 0.01);
        assert_eq!(c[3], 1.0);
    }

    #[test]
    fn parse_color_without_hash() {
        let c_hash = Config::parse_color("#4488FF");
        let c_no_hash = Config::parse_color("4488FF");
        assert_eq!(c_hash, c_no_hash);
    }

    #[test]
    fn parse_color_invalid_falls_back() {
        let c = Config::parse_color("#ZZZZZZ");
        assert!((c[0] - 128.0 / 255.0).abs() < 0.01);
        assert_eq!(c[3], 1.0);
    }

    #[test]
    fn test_default_config_valid() {
        let cfg = Config::default();
        assert!(cfg.torus_R > 0.0, "torus_R must be positive");
        assert!(cfg.torus_r > 0.0, "torus_r must be positive");
        assert!(cfg.time_step > 0.0, "time_step must be positive");
        assert!(
            cfg.rotation_speed >= 0.0,
            "rotation_speed must be non-negative"
        );
        assert!(
            !cfg.color_palette.is_empty(),
            "color_palette must not be empty"
        );
        assert!(cfg.trail_length > 0, "trail_length must be > 0");
        let known = ["torus", "sphere", "saddle", "enneper", "catenoid"];
        assert!(
            known.contains(&cfg.surface.as_str()),
            "unexpected default surface: {}",
            cfg.surface
        );
        assert!(cfg.validate().is_empty(), "default config should have no warnings");
    }

    #[test]
    fn test_config_round_trip() {
        let original = Config::default();
        let toml_str = toml::to_string(&original).expect("serialization failed");
        let restored: Config = toml::from_str(&toml_str).expect("deserialization failed");

        assert_eq!(original.surface, restored.surface);
        assert_eq!(original.num_geodesics, restored.num_geodesics);
        assert_eq!(original.trail_length, restored.trail_length);
        assert!((original.rotation_speed - restored.rotation_speed).abs() < 1e-9);
        assert_eq!(original.color_palette, restored.color_palette);
        assert!((original.torus_R - restored.torus_R).abs() < 1e-9);
        assert!((original.torus_r - restored.torus_r).abs() < 1e-9);
        assert!((original.time_step - restored.time_step).abs() < 1e-9);
        assert_eq!(original.show_wireframe, restored.show_wireframe);
        assert_eq!(original.max_trail_verts, restored.max_trail_verts);
        assert!((original.trail_fade_power - restored.trail_fade_power).abs() < 1e-9);
        assert_eq!(original.color_mode, restored.color_mode);
        assert_eq!(original.target_fps, restored.target_fps);
    }

    #[test]
    fn test_config_geodesic_count_nonzero() {
        let cfg = Config::default();
        assert!(
            cfg.num_geodesics > 0,
            "num_geodesics must be > 0, got {}",
            cfg.num_geodesics
        );
    }

    #[test]
    fn validate_catches_zero_geodesics() {
        let mut cfg = Config::default();
        cfg.num_geodesics = 0;
        let warnings = cfg.validate();
        assert!(
            warnings.iter().any(|w| w.contains("num_geodesics")),
            "expected warning about num_geodesics, got: {warnings:?}"
        );
    }

    #[test]
    fn validate_catches_self_intersecting_torus() {
        let mut cfg = Config::default();
        cfg.torus_r = 3.0;
        cfg.torus_R = 2.0;
        let warnings = cfg.validate();
        assert!(
            warnings.iter().any(|w| w.contains("self-intersect")),
            "expected self-intersect warning, got: {warnings:?}"
        );
    }

    #[test]
    fn validate_catches_empty_palette() {
        let mut cfg = Config::default();
        cfg.color_palette = vec![];
        let warnings = cfg.validate();
        assert!(
            warnings.iter().any(|w| w.contains("color_palette")),
            "expected palette warning, got: {warnings:?}"
        );
    }

    #[test]
    fn effective_target_fps_clamps_zero() {
        let mut cfg = Config::default();
        cfg.target_fps = 0;
        assert_eq!(cfg.effective_target_fps(), 30);
        cfg.target_fps = 60;
        assert_eq!(cfg.effective_target_fps(), 60);
    }

    #[test]
    fn effective_fade_power_clamps_nonpositive() {
        let mut cfg = Config::default();
        cfg.trail_fade_power = 0.0;
        assert!((cfg.effective_fade_power() - 2.0).abs() < 1e-6);
        cfg.trail_fade_power = 3.0;
        assert!((cfg.effective_fade_power() - 3.0).abs() < 1e-6);
    }
}
