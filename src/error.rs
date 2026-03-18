//! Application-level error types for geodesic-wallpaper.
//!
//! All fallible operations in the public API return `Result<_, GeodesicError>`.
//! The `thiserror` crate drives `Display` and `From` implementations so
//! callers get readable messages without a Rust backtrace dump.

use thiserror::Error;

/// Top-level error enum covering every subsystem that can fail.
///
/// # Examples
///
/// ```
/// use geodesic_wallpaper::error::GeodesicError;
///
/// let e = GeodesicError::config("config.toml missing");
/// assert!(e.to_string().contains("config.toml missing"));
/// ```
#[derive(Debug, Error)]
pub enum GeodesicError {
    /// A configuration file could not be read or parsed.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// The wgpu render pipeline failed to initialise or submit a frame.
    #[error("Render error: {0}")]
    RenderError(String),

    /// A surface parameterization produced a degenerate or invalid state.
    #[error("Surface error: {0}")]
    SurfaceError(String),

    /// Win32 window creation or management failed.
    #[error("Window error: {0}")]
    WindowError(String),
}

impl GeodesicError {
    /// Convenience constructor for [`GeodesicError::ConfigError`].
    pub fn config(msg: impl Into<String>) -> Self {
        GeodesicError::ConfigError(msg.into())
    }

    /// Convenience constructor for [`GeodesicError::RenderError`].
    pub fn render(msg: impl Into<String>) -> Self {
        GeodesicError::RenderError(msg.into())
    }

    /// Convenience constructor for [`GeodesicError::SurfaceError`].
    pub fn surface(msg: impl Into<String>) -> Self {
        GeodesicError::SurfaceError(msg.into())
    }

    /// Convenience constructor for [`GeodesicError::WindowError`].
    pub fn window(msg: impl Into<String>) -> Self {
        GeodesicError::WindowError(msg.into())
    }
}
