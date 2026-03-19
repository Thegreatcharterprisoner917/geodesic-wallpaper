//! Enneper minimal surface parameterization with analytic Christoffel symbols.

use super::Surface;
use glam::Vec3;
use rand::Rng;

/// The Enneper minimal surface.
///
/// # Parameterization
/// - `u, v ∈ [-2, 2]`
///
/// ```text
/// x = u − u³/3 + u·v²
/// y = v − v³/3 + v·u²
/// z = u² − v²
/// ```
///
/// This is a complete minimal surface of total curvature −4π. It self-intersects
/// for large |u|, |v| but is mathematically valid throughout the domain.
pub struct Enneper {
    /// Domain half-extent; parameters are clamped to `[−extent, extent]`.
    pub extent: f32,
}

impl Enneper {
    /// Create an Enneper surface with the given domain extent.
    ///
    /// The default extent of `1.5` avoids the worst self-intersections while
    /// showing the characteristic saddle shape.
    pub fn new(extent: f32) -> Self {
        Self {
            extent: extent.max(0.1),
        }
    }

    fn du(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(1.0 - u * u + v * v, 2.0 * u * v, 2.0 * u)
    }

    fn dv(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(2.0 * u * v, 1.0 - v * v + u * u, -2.0 * v)
    }

    fn d2u2(&self, _u: f32, _v: f32) -> Vec3 {
        // ∂²x/∂u² = (−2u, 2v, 2) → already computed inline
        Vec3::new(-2.0, 0.0, 2.0)
    }

    fn d2v2(&self, _u: f32, _v: f32) -> Vec3 {
        Vec3::new(0.0, -2.0, -2.0)
    }

    fn d2uv(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(2.0 * v, 2.0 * u, 0.0)
    }
}

impl Surface for Enneper {
    fn position(&self, u: f32, v: f32) -> Vec3 {
        Vec3::new(
            u - u * u * u / 3.0 + u * v * v,
            v - v * v * v / 3.0 + v * u * u,
            u * u - v * v,
        )
    }

    fn metric(&self, u: f32, v: f32) -> [[f32; 2]; 2] {
        let eu = self.du(u, v);
        let ev = self.dv(u, v);
        [
            [eu.dot(eu), eu.dot(ev)],
            [ev.dot(eu), ev.dot(ev)],
        ]
    }

    fn christoffel(&self, u: f32, v: f32) -> [[[f32; 2]; 2]; 2] {
        let eu = self.du(u, v);
        let ev = self.dv(u, v);
        let euu = self.d2u2(u, v);
        let evv = self.d2v2(u, v);
        let euv = self.d2uv(u, v);

        let g00 = eu.dot(eu);
        let g01 = eu.dot(ev);
        let g11 = ev.dot(ev);
        let det = g00 * g11 - g01 * g01;
        let inv_det = if det.abs() > 1e-12 { 1.0 / det } else { 0.0 };
        // g^{ij}: inverse of [[g00,g01],[g01,g11]]
        let gi00 = g11 * inv_det;
        let gi01 = -g01 * inv_det;
        let gi11 = g00 * inv_det;

        // Second-kind Christoffel: Γ^k_{ij} = g^{kl} <∂²φ/∂xᵢ∂xⱼ, ∂φ/∂xₗ>
        // Γ^k_{ij} = Σ_l g^{kl} <Φ_{ij}, e_l>
        let sym_uu_0 = euu.dot(eu);
        let sym_uu_1 = euu.dot(ev);
        let sym_vv_0 = evv.dot(eu);
        let sym_vv_1 = evv.dot(ev);
        let sym_uv_0 = euv.dot(eu);
        let sym_uv_1 = euv.dot(ev);

        let g0_uu = gi00 * sym_uu_0 + gi01 * sym_uu_1;
        let g1_uu = gi01 * sym_uu_0 + gi11 * sym_uu_1;
        let g0_vv = gi00 * sym_vv_0 + gi01 * sym_vv_1;
        let g1_vv = gi01 * sym_vv_0 + gi11 * sym_vv_1;
        let g0_uv = gi00 * sym_uv_0 + gi01 * sym_uv_1;
        let g1_uv = gi01 * sym_uv_0 + gi11 * sym_uv_1;

        // [k][i][j]
        [
            [[g0_uu, g0_uv], [g0_uv, g0_vv]],
            [[g1_uu, g1_uv], [g1_uv, g1_vv]],
        ]
    }

    fn wrap(&self, u: f32, v: f32) -> (f32, f32) {
        (
            u.clamp(-self.extent, self.extent),
            v.clamp(-self.extent, self.extent),
        )
    }

    fn normal(&self, u: f32, v: f32) -> Vec3 {
        let eu = self.du(u, v);
        let ev = self.dv(u, v);
        let n = eu.cross(ev);
        let len = n.length();
        if len > 1e-12 { n / len } else { Vec3::Z }
    }

    fn random_position(&self, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        let e = self.extent;
        (
            rng.gen_range(-e..e),
            rng.gen_range(-e..e),
        )
    }

    fn random_tangent(&self, u: f32, v: f32, rng: &mut dyn rand::RngCore) -> (f32, f32) {
        use std::f32::consts::TAU;
        let angle: f32 = rng.gen_range(0.0..TAU);
        let du = angle.cos();
        let dv = angle.sin();
        let g = self.metric(u, v);
        let speed_sq = g[0][0] * du * du + 2.0 * g[0][1] * du * dv + g[1][1] * dv * dv;
        if speed_sq > 1e-12 {
            let s = speed_sq.sqrt();
            (du / s, dv / s)
        } else {
            (1.0, 0.0)
        }
    }

    fn mesh_vertices(&self, u_steps: u32, v_steps: u32) -> (Vec<[f32; 3]>, Vec<u32>) {
        let mut verts = Vec::new();
        let mut indices = Vec::new();
        let e = self.extent;
        for vi in 0..=v_steps {
            for ui in 0..=u_steps {
                let u = -e + 2.0 * e * ui as f32 / u_steps as f32;
                let v = -e + 2.0 * e * vi as f32 / v_steps as f32;
                let p = self.position(u, v);
                verts.push([p.x, p.y, p.z]);
            }
        }
        let cols = u_steps + 1;
        for vi in 0..v_steps {
            for ui in 0..u_steps {
                let a = vi * cols + ui;
                let b = a + 1;
                let c = a + cols;
                let d = c + 1;
                indices.extend_from_slice(&[a, b, c, b, d, c]);
            }
        }
        (verts, indices)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enneper_position_at_origin() {
        let s = Enneper::new(1.5);
        let p = s.position(0.0, 0.0);
        assert!(p.length() < 1e-6, "position at origin should be zero: {p:?}");
    }

    #[test]
    fn enneper_metric_positive_definite() {
        let s = Enneper::new(1.5);
        for &u in &[0.1f32, 0.5, 1.0] {
            for &v in &[0.1f32, 0.5, 1.0] {
                let g = s.metric(u, v);
                let det = g[0][0] * g[1][1] - g[0][1] * g[1][0];
                assert!(g[0][0] > 0.0, "g00 not positive at ({u},{v})");
                assert!(det > 0.0, "metric not positive definite at ({u},{v})");
            }
        }
    }

    #[test]
    fn enneper_christoffel_finite() {
        let s = Enneper::new(1.5);
        let g = s.christoffel(0.5, 0.3);
        for k in 0..2 {
            for i in 0..2 {
                for j in 0..2 {
                    assert!(
                        g[k][i][j].is_finite(),
                        "Γ^{k}_{i}{j} not finite: {}",
                        g[k][i][j]
                    );
                }
            }
        }
    }
}
