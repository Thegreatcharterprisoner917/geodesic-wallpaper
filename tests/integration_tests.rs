//! Integration tests for geodesic-wallpaper.
#![allow(clippy::needless_range_loop, clippy::unwrap_used)]
//!
//! Tests cover:
//! - Geodesic integration correctness on flat torus and sphere.
//! - RK4 conservation of the metric speed invariant.
//! - Trail ring-buffer correctness and fade logic.
//! - Config parsing edge cases.

// ---------------------------------------------------------------------------
// Helpers -- inline minimal surface implementations so tests do not depend
// on any Win32 or wgpu code paths.
// ---------------------------------------------------------------------------

/// A flat torus (zero curvature in both parameter directions).
///
/// Parameterization: u in [0, 1), v in [0, 1).
/// The embedding is just the identity R^2 map, so Christoffel symbols are
/// all zero and geodesics are straight lines in (u, v) space.
struct FlatTorus;

impl FlatTorus {
    /// Metric tensor: identity (flat).
    fn metric(&self) -> [[f32; 2]; 2] {
        [[1.0, 0.0], [0.0, 1.0]]
    }

    /// All Christoffel symbols vanish on a flat torus.
    fn christoffel(&self) -> [[[f32; 2]; 2]; 2] {
        [[[0.0; 2]; 2]; 2]
    }

    /// Step one RK4 iteration on the geodesic ODE.
    fn rk4_step(&self, u: f32, v: f32, du: f32, dv: f32, dt: f32) -> (f32, f32, f32, f32) {
        let deriv = |_u: f32, _v: f32, du: f32, dv: f32| -> (f32, f32, f32, f32) {
            let g = self.christoffel();
            let acc_u = -(g[0][0][0] * du * du + 2.0 * g[0][0][1] * du * dv + g[0][1][1] * dv * dv);
            let acc_v = -(g[1][0][0] * du * du + 2.0 * g[1][0][1] * du * dv + g[1][1][1] * dv * dv);
            (du, dv, acc_u, acc_v)
        };
        let (k1u, k1v, k1du, k1dv) = deriv(u, v, du, dv);
        let (k2u, k2v, k2du, k2dv) = deriv(
            u + 0.5 * dt * k1u,
            v + 0.5 * dt * k1v,
            du + 0.5 * dt * k1du,
            dv + 0.5 * dt * k1dv,
        );
        let (k3u, k3v, k3du, k3dv) = deriv(
            u + 0.5 * dt * k2u,
            v + 0.5 * dt * k2v,
            du + 0.5 * dt * k2du,
            dv + 0.5 * dt * k2dv,
        );
        let (k4u, k4v, k4du, k4dv) =
            deriv(u + dt * k3u, v + dt * k3v, du + dt * k3du, dv + dt * k3dv);
        let new_u = u + dt / 6.0 * (k1u + 2.0 * k2u + 2.0 * k3u + k4u);
        let new_v = v + dt / 6.0 * (k1v + 2.0 * k2v + 2.0 * k3v + k4v);
        let new_du = du + dt / 6.0 * (k1du + 2.0 * k2du + 2.0 * k3du + k4du);
        let new_dv = dv + dt / 6.0 * (k1dv + 2.0 * k2dv + 2.0 * k3dv + k4dv);
        (new_u, new_v, new_du, new_dv)
    }
}

// ---------------------------------------------------------------------------
// Test module
// ---------------------------------------------------------------------------

/// Compute metric speed g_ij du^i du^j given the 2x2 metric and velocity.
fn metric_speed(g: [[f32; 2]; 2], du: f32, dv: f32) -> f32 {
    g[0][0] * du * du + 2.0 * g[0][1] * du * dv + g[1][1] * dv * dv
}

// ---- Flat torus geodesic tests ----------------------------------------

#[test]
fn flat_torus_geodesic_is_straight_line_u_direction() {
    // A geodesic with initial velocity (1, 0) on a flat torus must remain
    // a horizontal line: v must not change.
    let surf = FlatTorus;
    let (mut u, mut v, mut du, mut dv) = (0.0f32, 0.5f32, 1.0f32, 0.0f32);
    let dt = 0.01f32;
    for _ in 0..200 {
        let (nu, nv, ndu, ndv) = surf.rk4_step(u, v, du, dv, dt);
        (u, v, du, dv) = (nu, nv, ndu, ndv);
    }
    // v must be unchanged to within floating-point round-off accumulated over 200 steps.
    assert!(
        (v - 0.5).abs() < 1e-5,
        "v deviated from 0.5 on flat torus: v = {v}"
    );
    // dv must remain zero.
    assert!(dv.abs() < 1e-5, "dv non-zero on flat torus: dv = {dv}");
}

#[test]
fn flat_torus_geodesic_is_straight_line_diagonal() {
    // A geodesic with velocity (1, 1) must trace u = v.
    let surf = FlatTorus;
    let (mut u, mut v, mut du, mut dv) = (0.0f32, 0.0f32, 1.0f32, 1.0f32);
    let dt = 0.01f32;
    for _ in 0..100 {
        let (nu, nv, ndu, ndv) = surf.rk4_step(u, v, du, dv, dt);
        (u, v, du, dv) = (nu, nv, ndu, ndv);
    }
    // After N steps both coordinates advance identically.
    assert!(
        (u - v).abs() < 1e-4,
        "u and v diverged on flat torus diagonal geodesic: u={u} v={v}"
    );
}

#[test]
fn flat_torus_rk4_conserves_metric_speed() {
    // On a flat torus the metric speed g_ij du^i du^j must be constant because
    // all Christoffel symbols vanish and the velocity is constant.
    let surf = FlatTorus;
    let g = surf.metric();
    let (mut u, mut v, mut du, mut dv) = (0.3f32, 0.7f32, 0.6f32, 0.8f32);
    let initial_speed = metric_speed(g, du, dv);
    let dt = 0.01f32;
    for _ in 0..500 {
        let (nu, nv, ndu, ndv) = surf.rk4_step(u, v, du, dv, dt);
        (u, v, du, dv) = (nu, nv, ndu, ndv);
    }
    let final_speed = metric_speed(g, du, dv);
    assert!(
        (final_speed - initial_speed).abs() < 1e-4,
        "metric speed not conserved: initial={initial_speed} final={final_speed}"
    );
}

#[test]
fn flat_torus_velocity_unchanged_after_many_steps() {
    // On a flat surface, the geodesic equation has zero acceleration;
    // velocity must be numerically unchanged.
    let surf = FlatTorus;
    let (mut u, mut v, mut du, mut dv) = (0.0f32, 0.0f32, 0.5f32, -0.3f32);
    let initial_du = du;
    let initial_dv = dv;
    let dt = 0.01f32;
    for _ in 0..1000 {
        let (nu, nv, ndu, ndv) = surf.rk4_step(u, v, du, dv, dt);
        (u, v, du, dv) = (nu, nv, ndu, ndv);
    }
    assert!(
        (du - initial_du).abs() < 1e-4,
        "du changed on flat surface: {du}"
    );
    assert!(
        (dv - initial_dv).abs() < 1e-4,
        "dv changed on flat surface: {dv}"
    );
}

// ---- Torus Christoffel symbol symmetry test ----------------------------

/// Simplified Torus Christoffel computation to verify symmetry without wgpu.
struct SimpleTorus {
    big_r: f32,
    small_r: f32,
}

impl SimpleTorus {
    fn christoffel(&self, v: f32) -> [[[f32; 2]; 2]; 2] {
        let f = self.big_r + self.small_r * v.cos();
        let df_dv = -self.small_r * v.sin();
        let g11 = self.small_r * self.small_r;
        let gamma_0_01 = df_dv / f;
        let gamma_1_00 = -f * df_dv / g11;
        [
            [[0.0, gamma_0_01], [gamma_0_01, 0.0]],
            [[gamma_1_00, 0.0], [0.0, 0.0]],
        ]
    }
}

#[test]
fn torus_christoffel_is_symmetric_in_lower_indices() {
    // For any torsion-free connection, Gamma^k_ij = Gamma^k_ji.
    let torus = SimpleTorus {
        big_r: 2.0,
        small_r: 0.7,
    };
    for &v in &[0.0f32, 0.5, 1.0, 2.0, 3.0, 5.0] {
        let g = torus.christoffel(v);
        for k in 0..2 {
            for i in 0..2 {
                for j in 0..2 {
                    let diff = (g[k][i][j] - g[k][j][i]).abs();
                    assert!(
                        diff < 1e-6,
                        "Gamma^{k}_{i}{j} != Gamma^{k}_{j}{i} at v={v}: diff={diff}"
                    );
                }
            }
        }
    }
}

#[test]
fn torus_christoffel_zero_on_outer_equator() {
    // At v = 0 (outer equator), sin(v) = 0, so df_dv = 0 and all Christoffels vanish.
    let torus = SimpleTorus {
        big_r: 2.0,
        small_r: 0.7,
    };
    let g = torus.christoffel(0.0);
    for k in 0..2 {
        for i in 0..2 {
            for j in 0..2 {
                assert!(
                    g[k][i][j].abs() < 1e-5,
                    "Non-zero Christoffel at v=0: Gamma^{k}_{i}{j} = {}",
                    g[k][i][j]
                );
            }
        }
    }
}

#[test]
fn torus_christoffel_nonzero_off_equator() {
    // At v = pi/2, sin(v) = 1 != 0, so the (0,01) and (1,00) components are nonzero.
    let torus = SimpleTorus {
        big_r: 2.0,
        small_r: 0.7,
    };
    let g = torus.christoffel(std::f32::consts::FRAC_PI_2);
    // gamma_0_01 = -small_r / (big_r) = -0.7/2.0 = -0.35
    assert!(
        g[0][0][1].abs() > 0.1,
        "Expected nonzero Gamma^0_01 off equator, got {}",
        g[0][0][1]
    );
    // gamma_1_00 = big_r * small_r / small_r^2 = big_r / small_r
    assert!(
        g[1][0][0].abs() > 0.1,
        "Expected nonzero Gamma^1_00 off equator, got {}",
        g[1][0][0]
    );
}

// ---- Trail buffer tests ------------------------------------------------

/// Minimal TrailBuffer reimplementation for isolated testing.
struct TrailBuf {
    data: Vec<[f32; 3]>,
    head: usize,
    count: usize,
    cap: usize,
}

impl TrailBuf {
    fn new(cap: usize) -> Self {
        Self {
            data: vec![[0.0; 3]; cap],
            head: 0,
            count: 0,
            cap,
        }
    }

    fn push(&mut self, pos: [f32; 3]) {
        self.data[self.head] = pos;
        self.head = (self.head + 1) % self.cap;
        if self.count < self.cap {
            self.count += 1;
        }
    }

    fn clear(&mut self) {
        self.count = 0;
        self.head = 0;
    }

    fn ordered(&self) -> Vec<([f32; 3], f32)> {
        let mut out = Vec::with_capacity(self.count);
        for i in 0..self.count {
            let age_frac = i as f32 / self.count.max(1) as f32;
            let alpha = age_frac * age_frac;
            let idx = if self.count == self.cap {
                (self.head + i) % self.cap
            } else {
                i
            };
            out.push((self.data[idx], alpha));
        }
        out
    }
}

#[test]
fn trail_buffer_push_increases_count() {
    let mut buf = TrailBuf::new(10);
    assert_eq!(buf.count, 0);
    buf.push([1.0, 2.0, 3.0]);
    assert_eq!(buf.count, 1);
    buf.push([4.0, 5.0, 6.0]);
    assert_eq!(buf.count, 2);
}

#[test]
fn trail_buffer_count_capped_at_capacity() {
    let mut buf = TrailBuf::new(5);
    for i in 0..20 {
        buf.push([i as f32, 0.0, 0.0]);
    }
    assert_eq!(buf.count, 5, "count must not exceed capacity");
}

#[test]
fn trail_buffer_clear_resets_state() {
    let mut buf = TrailBuf::new(8);
    for i in 0..8 {
        buf.push([i as f32, 0.0, 0.0]);
    }
    buf.clear();
    assert_eq!(buf.count, 0);
    assert_eq!(buf.head, 0);
}

#[test]
fn trail_buffer_oldest_vertex_has_zero_alpha() {
    let mut buf = TrailBuf::new(10);
    for i in 0..10 {
        buf.push([i as f32, 0.0, 0.0]);
    }
    let verts = buf.ordered();
    // Oldest entry (index 0) has age_frac = 0 so alpha = 0.
    let (_, alpha) = verts[0];
    assert!(
        alpha.abs() < 1e-6,
        "oldest vertex alpha should be 0, got {alpha}"
    );
}

#[test]
fn trail_buffer_newest_vertex_has_near_full_alpha() {
    let mut buf = TrailBuf::new(100);
    for i in 0..100 {
        buf.push([i as f32, 0.0, 0.0]);
    }
    let verts = buf.ordered();
    // Newest entry: age_frac = 99/99 = 1, alpha = 1^2 = 1.
    let (_, alpha) = *verts.last().unwrap();
    assert!(
        (alpha - 1.0).abs() < 0.02,
        "newest vertex alpha should be ~1, got {alpha}"
    );
}

#[test]
fn trail_buffer_alpha_is_monotonically_increasing() {
    let mut buf = TrailBuf::new(50);
    for i in 0..50 {
        buf.push([i as f32, 0.0, 0.0]);
    }
    let verts = buf.ordered();
    for w in verts.windows(2) {
        let (_, a0) = w[0];
        let (_, a1) = w[1];
        assert!(
            a1 >= a0,
            "alpha not monotonically increasing: {a0} then {a1}"
        );
    }
}

#[test]
fn trail_buffer_ring_wrap_returns_correct_order() {
    // Fill a capacity-4 buffer with values 0..7 so it wraps twice.
    let mut buf = TrailBuf::new(4);
    for i in 0u32..8 {
        buf.push([i as f32, 0.0, 0.0]);
    }
    // After 8 pushes to a cap-4 buffer, the oldest is 4, newest is 7.
    let verts = buf.ordered();
    assert_eq!(verts.len(), 4);
    assert!(
        (verts[0].0[0] - 4.0).abs() < 1e-5,
        "oldest should be 4, got {}",
        verts[0].0[0]
    );
    assert!(
        (verts[3].0[0] - 7.0).abs() < 1e-5,
        "newest should be 7, got {}",
        verts[3].0[0]
    );
}

// ---- Config parsing edge-case tests ------------------------------------
// These replicate the logic in config.rs without importing the module
// (which pulls in Win32 dependencies).

fn parse_color(hex: &str) -> [f32; 4] {
    let h = hex.trim_start_matches('#');
    let r = u8::from_str_radix(h.get(0..2).unwrap_or("80"), 16).unwrap_or(128) as f32 / 255.0;
    let g = u8::from_str_radix(h.get(2..4).unwrap_or("80"), 16).unwrap_or(128) as f32 / 255.0;
    let b = u8::from_str_radix(h.get(4..6).unwrap_or("80"), 16).unwrap_or(128) as f32 / 255.0;
    [r, g, b, 1.0]
}

#[test]
fn config_color_white() {
    let c = parse_color("#FFFFFF");
    for ch in [c[0], c[1], c[2]] {
        assert!(
            (ch - 1.0).abs() < 1e-3,
            "channel should be 1.0 for white, got {ch}"
        );
    }
    assert_eq!(c[3], 1.0);
}

#[test]
fn config_color_black() {
    let c = parse_color("#000000");
    for ch in [c[0], c[1], c[2]] {
        assert!(ch.abs() < 1e-6, "channel should be 0.0 for black, got {ch}");
    }
}

#[test]
fn config_color_without_hash_equals_with_hash() {
    let with_hash = parse_color("#4488FF");
    let without_hash = parse_color("4488FF");
    for i in 0..4 {
        assert!((with_hash[i] - without_hash[i]).abs() < 1e-6);
    }
}

#[test]
fn config_color_too_short_uses_fallback() {
    // A 2-character string: first channel parsed, rest fall back to 128.
    let c = parse_color("FF");
    assert!((c[0] - 1.0).abs() < 1e-3);
    // Remaining channels fall back to 128/255 ~ 0.502.
    assert!((c[1] - 128.0 / 255.0).abs() < 1e-3);
}

#[test]
fn config_color_invalid_hex_digits_use_fallback() {
    let c = parse_color("#ZZZZZZ");
    // All three channels should fall back to 128/255.
    for ch in [c[0], c[1], c[2]] {
        assert!(
            (ch - 128.0 / 255.0).abs() < 1e-3,
            "expected 128/255 fallback, got {ch}"
        );
    }
    assert_eq!(c[3], 1.0);
}

#[test]
fn config_color_alpha_always_one() {
    for hex in ["#FF0000", "#00FF00", "#0000FF", "#FFFFFF", "#000000"] {
        let c = parse_color(hex);
        assert_eq!(c[3], 1.0, "alpha should be 1.0 for {hex}");
    }
}
