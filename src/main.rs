//! Entry point for geodesic-wallpaper.
//!
//! Initialises logging and tracing, loads configuration, spawns the hot-reload
//! watcher, creates the Win32 wallpaper window, builds the wgpu renderer, and
//! runs the main message/render loop.

use geodesic_wallpaper::config::{Config, SharedConfig};
use geodesic_wallpaper::error::GeodesicError;
use geodesic_wallpaper::geodesic::Geodesic;
use geodesic_wallpaper::renderer::Renderer;
use geodesic_wallpaper::surface::{
    catenoid::Catenoid, enneper::Enneper, saddle::Saddle, sphere::Sphere, torus::Torus, Surface,
};
use geodesic_wallpaper::trail::TrailBuffer;
use geodesic_wallpaper::tray;
use geodesic_wallpaper::wallpaper;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, MessageBoxW, PeekMessageW, TranslateMessage, MSG, MB_ICONWARNING, MB_OK,
    PM_REMOVE,
};

/// Construct the surface implementation selected in `cfg`.
///
/// Unrecognised surface names fall back to the torus.
#[tracing::instrument(skip(cfg), fields(surface = %cfg.surface))]
fn build_surface(cfg: &Config) -> Box<dyn Surface> {
    tracing::info!(surface = %cfg.surface, "building surface");
    match cfg.surface.as_str() {
        "sphere" => Box::new(Sphere::new(2.5)),
        "saddle" => Box::new(Saddle::new(2.0)),
        "enneper" => Box::new(Enneper::new(1.5)),
        "catenoid" => Box::new(Catenoid::new(1.5)),
        _ => Box::new(Torus::new(cfg.torus_R, cfg.torus_r)),
    }
}

/// Convert the hex colour palette from config into `[f32; 4]` RGBA values.
fn parse_colors(cfg: &Config) -> Vec<[f32; 4]> {
    if cfg.color_palette.is_empty() {
        return vec![[0.5, 0.5, 0.5, 1.0]];
    }
    cfg.color_palette
        .iter()
        .map(|s| Config::parse_color(s))
        .collect()
}

/// Pick a color index for geodesic `i` given the color mode.
fn pick_color_idx(i: usize, color_count: usize, mode: &str, rng: &mut StdRng) -> usize {
    if mode == "random" {
        rng.gen_range(0..color_count.max(1))
    } else {
        i % color_count.max(1)
    }
}

/// Query the primary monitor resolution via Win32.
///
/// Returns `(1920, 1080)` as a safe fallback if the system call fails.
fn screen_size(multi_monitor: bool) -> (i32, i32) {
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
        };
        if multi_monitor {
            let w = GetSystemMetrics(SM_CXVIRTUALSCREEN);
            let h = GetSystemMetrics(SM_CYVIRTUALSCREEN);
            if w > 0 && h > 0 {
                return (w, h);
            }
        }
        let w = GetSystemMetrics(SM_CXSCREEN);
        let h = GetSystemMetrics(SM_CYSCREEN);
        if w > 0 && h > 0 { (w, h) } else { (1920, 1080) }
    }
}

/// Show an epilepsy/photosensitivity warning using a Win32 message box.
fn show_epilepsy_warning() {
    unsafe {
        let text: Vec<u16> = "Geodesic Wallpaper displays animated moving patterns.\n\n\
            If you or anyone in your household is photosensitive or has a history \
            of epilepsy, please be aware that the animated visuals may trigger \
            symptoms.\n\nPress OK to continue."
            .encode_utf16()
            .chain(std::iter::once(0u16))
            .collect();
        let caption: Vec<u16> = "Photosensitivity Notice"
            .encode_utf16()
            .chain(std::iter::once(0u16))
            .collect();
        let _ = MessageBoxW(
            None,
            windows::core::PCWSTR(text.as_ptr()),
            windows::core::PCWSTR(caption.as_ptr()),
            MB_ICONWARNING | MB_OK,
        );
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    if let Err(e) = run() {
        tracing::error!("Fatal error: {e}");
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

/// Application body returning a typed error on failure.
#[tracing::instrument]
fn run() -> Result<(), GeodesicError> {
    let config_path = PathBuf::from("config.toml");
    let cfg = Config::load(&config_path);
    tracing::info!(
        surface = %cfg.surface,
        num_geodesics = cfg.num_geodesics,
        trail_length = cfg.trail_length,
        target_fps = cfg.target_fps,
        multi_monitor = cfg.multi_monitor,
        "configuration loaded"
    );

    // Epilepsy warning on startup (can be disabled in config).
    if cfg.epilepsy_warning {
        show_epilepsy_warning();
    }

    let shared_cfg: SharedConfig = Arc::new(RwLock::new(cfg.clone()));

    // Spawn hot-reload watcher thread.
    {
        let shared = shared_cfg.clone();
        let path = config_path.clone();
        std::thread::spawn(move || {
            use notify::{recommended_watcher, RecursiveMode, Watcher};
            let (tx, rx) = std::sync::mpsc::channel();
            let mut watcher = match recommended_watcher(move |res| {
                let _ = tx.send(res);
            }) {
                Ok(w) => w,
                Err(e) => {
                    tracing::warn!("Failed to start config watcher: {e}");
                    return;
                }
            };
            let _ = watcher.watch(&path, RecursiveMode::NonRecursive);
            loop {
                if rx.recv().is_ok() {
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    let new_cfg = Config::load(&path);
                    if let Ok(mut w) = shared.write() {
                        *w = new_cfg;
                        tracing::info!("config reloaded from disk");
                    }
                }
            }
        });
    }

    // Spawn system tray icon.
    let tray_state = tray::spawn_tray(cfg.surface.clone());

    let (sw, sh) = screen_size(cfg.multi_monitor);
    tracing::info!(width = sw, height = sh, "detected screen resolution");
    let hwnd = wallpaper::create_wallpaper_hwnd(sw, sh)
        .ok_or_else(|| GeodesicError::window("Failed to create wallpaper window"))?;

    let mut surf = build_surface(&cfg);
    let colors = parse_colors(&cfg);
    let fade_power = cfg.effective_fade_power();
    let (mesh_verts, mesh_indices) = surf.mesh_vertices(40, 40);
    tracing::info!(
        verts = mesh_verts.len(),
        indices = mesh_indices.len(),
        "surface mesh generated"
    );

    let mut renderer = pollster::block_on(Renderer::new(
        hwnd,
        sw as u32,
        sh as u32,
        &mesh_verts,
        &mesh_indices,
        cfg.max_trail_verts,
        cfg.show_wireframe,
    ))?;

    // Set camera from config.
    renderer.camera = geodesic_wallpaper::renderer::camera::Camera::new_with_params(
        sw as f32 / sh as f32,
        cfg.camera_distance,
        cfg.camera_elevation,
        cfg.camera_fov,
        cfg.camera_elevation_speed,
    );

    let mut rng = StdRng::from_entropy();
    let mut geodesics: Vec<Geodesic> = Vec::new();
    let mut trails: Vec<TrailBuffer> = Vec::new();

    let color_mode = cfg.color_mode.clone();
    for i in 0..cfg.num_geodesics {
        let (u, v) = surf.random_position(&mut rng);
        let (du, dv) = surf.random_tangent(u, v, &mut rng);
        let ci = pick_color_idx(i, colors.len(), &color_mode, &mut rng);
        geodesics.push(Geodesic::new(u, v, du, dv, cfg.trail_length, ci));
        trails.push(TrailBuffer::new(cfg.trail_length, colors[ci], fade_power));
    }
    tracing::info!(count = cfg.num_geodesics, "geodesics spawned");

    let dt = cfg.time_step;
    let mut target_frame: std::time::Duration;
    let mut last_frame = std::time::Instant::now();

    // HUD tracking.
    let mut hud_last = std::time::Instant::now();
    let mut frame_count: u64 = 0;

    // Current surface name (may be changed by tray).
    let mut current_surface_name = cfg.surface.clone();

    tracing::info!("entering render loop");

    loop {
        // Check for tray-requested quit.
        if tray_state.quit_requested() {
            tracing::info!("quit requested via tray icon");
            return Ok(());
        }

        // Check for tray surface switch request.
        if let Some(new_surface) = tray_state.take_surface_request() {
            if new_surface != current_surface_name {
                tracing::info!(surface = %new_surface, "switching surface via tray");
                current_surface_name = new_surface.clone();
                // Update shared config surface field.
                if let Ok(mut w) = shared_cfg.write() {
                    w.surface = new_surface.clone();
                }
                surf = build_surface_by_name(&new_surface, &*shared_cfg.read().unwrap_or_else(|e| e.into_inner()));
                let (mv, mi) = surf.mesh_vertices(40, 40);
                // Rebuild renderer with new mesh (requires new GPU buffers).
                let show_wire = renderer.show_wireframe;
                let max_tv = renderer.trail_vbuf_capacity();
                renderer = pollster::block_on(Renderer::new(
                    hwnd,
                    sw as u32,
                    sh as u32,
                    &mv,
                    &mi,
                    max_tv,
                    show_wire,
                ))?;
                renderer.camera = geodesic_wallpaper::renderer::camera::Camera::new_with_params(
                    sw as f32 / sh as f32,
                    cfg.camera_distance,
                    cfg.camera_elevation,
                    cfg.camera_fov,
                    cfg.camera_elevation_speed,
                );
                // Respawn all geodesics on new surface.
                geodesics.clear();
                trails.clear();
                let (tl, ng, fp, cm) = {
                    let c = shared_cfg.read().map(|c| (c.trail_length, c.num_geodesics, c.effective_fade_power(), c.color_mode.clone())).unwrap_or((300, 30, 2.0, "cycle".into()));
                    c
                };
                for i in 0..ng {
                    let (u, v) = surf.random_position(&mut rng);
                    let (du, dv) = surf.random_tangent(u, v, &mut rng);
                    let ci = pick_color_idx(i, colors.len(), &cm, &mut rng);
                    geodesics.push(Geodesic::new(u, v, du, dv, tl, ci));
                    trails.push(TrailBuffer::new(tl, colors[ci], fp));
                }
            }
        }

        // Drain pending Win32 messages.
        unsafe {
            let mut msg = MSG::default();
            while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                if msg.message == 0x0012 {
                    // WM_QUIT
                    tracing::info!("WM_QUIT received, exiting");
                    return Ok(());
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        // Adaptive frame rate limit.
        {
            let fps = shared_cfg
                .read()
                .map(|c| c.effective_target_fps())
                .unwrap_or(30);
            target_frame = std::time::Duration::from_micros(1_000_000 / fps as u64);
        }
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(last_frame);
        if elapsed < target_frame {
            let remaining = target_frame - elapsed;
            if remaining > std::time::Duration::from_millis(1) {
                std::thread::sleep(remaining - std::time::Duration::from_micros(500));
            }
            continue;
        }
        let actual_dt = elapsed.as_secs_f32().min(0.1); // cap to avoid spiral on lag
        last_frame = std::time::Instant::now();
        frame_count += 1;

        // HUD: log FPS every second if enabled.
        {
            let show_hud = shared_cfg.read().map(|c| c.show_hud).unwrap_or(false);
            if show_hud {
                let hud_elapsed = hud_last.elapsed();
                if hud_elapsed >= std::time::Duration::from_secs(1) {
                    let fps = frame_count as f64 / hud_elapsed.as_secs_f64();
                    tracing::info!(
                        fps = format!("{fps:.1}"),
                        surface = %current_surface_name,
                        geodesics = geodesics.len(),
                        "HUD"
                    );
                    hud_last = std::time::Instant::now();
                    frame_count = 0;
                }
            }
        }

        // Skip stepping/rendering while paused.
        if tray_state.is_paused() {
            continue;
        }

        // Sync renderer wireframe toggle from config.
        {
            let show_wire = shared_cfg.read().map(|c| c.show_wireframe).unwrap_or(true);
            renderer.show_wireframe = show_wire;
        }

        // Orbit camera (azimuth).
        let rot = shared_cfg
            .read()
            .map(|c| c.rotation_speed)
            .unwrap_or(0.001047);
        renderer.camera.orbit(rot * actual_dt);
        renderer.camera.drift_elevation(actual_dt);

        // Step geodesics.
        for (i, geo) in geodesics.iter_mut().enumerate() {
            if !geo.alive {
                continue;
            }
            let pos = surf.position(geo.u, geo.v);
            trails[i].push([pos.x, pos.y, pos.z]);
            geo.step(surf.as_ref(), dt);
        }

        // Respawn dead geodesics.
        let (tl, cm, fp) = shared_cfg
            .read()
            .map(|c| (c.trail_length, c.color_mode.clone(), c.effective_fade_power()))
            .unwrap_or((300, "cycle".into(), 2.0));

        for (i, geo) in geodesics.iter_mut().enumerate() {
            if !geo.alive {
                let (u, v) = surf.random_position(&mut rng);
                let (du, dv) = surf.random_tangent(u, v, &mut rng);
                let ci = pick_color_idx(i, colors.len(), &cm, &mut rng);
                *geo = Geodesic::new(u, v, du, dv, tl, ci);
                trails[i].clear();
                trails[i].color = colors[ci];
                trails[i].fade_power = fp;
            }
        }

        // Collect trail vertices.
        let mut all_verts = Vec::new();
        let mut seg_lens = Vec::new();
        for trail in &trails {
            let v = trail.ordered_vertices();
            seg_lens.push(v.len());
            all_verts.extend(v);
        }

        renderer.render(&all_verts, &seg_lens);
    }
}

/// Build a surface by name string, reading torus parameters from config.
fn build_surface_by_name(name: &str, cfg: &Config) -> Box<dyn Surface> {
    match name {
        "sphere" => Box::new(Sphere::new(2.5)),
        "saddle" => Box::new(Saddle::new(2.0)),
        "enneper" => Box::new(Enneper::new(1.5)),
        "catenoid" => Box::new(Catenoid::new(1.5)),
        _ => Box::new(Torus::new(cfg.torus_R, cfg.torus_r)),
    }
}
