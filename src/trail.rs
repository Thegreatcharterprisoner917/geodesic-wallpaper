//! Ring-buffer trail storage for geodesic curves.
//!
//! Each active geodesic owns one [`TrailBuffer`] that records its most recent
//! 3D positions in a fixed-capacity ring buffer. When the buffer is full, the
//! oldest entry is overwritten. Alpha is computed per-vertex at read time so
//! the oldest samples are transparent and the newest are fully opaque.

use bytemuck::{Pod, Zeroable};

/// A single vertex in a rendered trail.
///
/// Stores the 3D world-space position and an RGBA colour. The alpha channel
/// is computed by [`TrailBuffer::ordered_vertices`] to produce a quadratic
/// fade from transparent (tail) to opaque (head).
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct TrailVertex {
    /// World-space position of this trail sample.
    pub position: [f32; 3],
    /// RGBA colour including fade alpha.
    pub color: [f32; 4],
}

/// Fixed-capacity ring buffer storing [`TrailVertex`] samples for one geodesic.
///
/// New positions are appended via [`TrailBuffer::push`]. When the buffer is
/// full the oldest sample is silently overwritten. Use
/// [`TrailBuffer::ordered_vertices`] to read samples in chronological order
/// with fade alpha applied.
pub struct TrailBuffer {
    /// Backing store; always allocated at full capacity.
    pub vertices: Vec<TrailVertex>,
    /// Index of the next write slot.
    pub head: usize,
    /// Number of valid samples currently stored (saturates at `capacity`).
    pub count: usize,
    /// Maximum number of samples stored (allocated capacity).
    pub capacity: usize,
    /// Base colour applied to every vertex before the fade alpha is computed.
    pub color: [f32; 4],
}

impl TrailBuffer {
    /// Allocate a new trail buffer with the given `capacity` and `color`.
    ///
    /// All slots are initialised to the zero position and zero alpha.
    pub fn new(capacity: usize, color: [f32; 4]) -> Self {
        // Guard against zero capacity to avoid divide-by-zero in ordered_vertices.
        let capacity = capacity.max(1);
        Self {
            vertices: vec![
                TrailVertex {
                    position: [0.0; 3],
                    color: [0.0; 4]
                };
                capacity
            ],
            head: 0,
            count: 0,
            capacity,
            color,
        }
    }

    /// Append a new 3D position sample to the trail.
    ///
    /// If the buffer is full the oldest sample is overwritten.
    pub fn push(&mut self, pos: [f32; 3]) {
        self.vertices[self.head] = TrailVertex {
            position: pos,
            color: self.color,
        };
        self.head = (self.head + 1) % self.capacity;
        if self.count < self.capacity {
            self.count += 1;
        }
    }

    /// Reset the buffer, discarding all stored samples.
    pub fn clear(&mut self) {
        self.count = 0;
        self.head = 0;
    }

    /// Return all stored samples in chronological order (oldest first) with
    /// quadratic fade alpha applied.
    ///
    /// Alpha is `(i / count)^2` where `i = 0` is the oldest sample and
    /// `i = count - 1` is the newest, making the tail transparent and the head
    /// fully opaque.
    pub fn ordered_vertices(&self) -> Vec<TrailVertex> {
        let mut out = Vec::with_capacity(self.count);
        for i in 0..self.count {
            let age_frac = i as f32 / self.count.max(1) as f32; // 0 = oldest, 1 = newest
            let alpha = age_frac * age_frac; // quadratic fade
            let idx = if self.count == self.capacity {
                (self.head + i) % self.capacity
            } else {
                i
            };
            let v = self.vertices[idx];
            out.push(TrailVertex {
                position: v.position,
                color: [v.color[0], v.color[1], v.color[2], alpha],
            });
        }
        out
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn red() -> [f32; 4] {
        [1.0, 0.0, 0.0, 1.0]
    }

    /// A freshly created buffer should report zero count.
    #[test]
    fn new_buffer_is_empty() {
        let buf = TrailBuffer::new(10, red());
        assert_eq!(buf.count, 0);
        assert_eq!(buf.ordered_vertices().len(), 0);
    }

    /// After filling the buffer beyond capacity the count saturates.
    #[test]
    fn count_saturates_at_capacity() {
        let cap = 5;
        let mut buf = TrailBuffer::new(cap, red());
        for i in 0..20 {
            buf.push([i as f32, 0.0, 0.0]);
        }
        assert_eq!(buf.count, cap);
        assert_eq!(buf.ordered_vertices().len(), cap);
    }

    /// After clear() the buffer should behave as if newly created.
    #[test]
    fn clear_resets_buffer() {
        let mut buf = TrailBuffer::new(8, red());
        for i in 0..8 {
            buf.push([i as f32, 0.0, 0.0]);
        }
        buf.clear();
        assert_eq!(buf.count, 0);
        assert_eq!(buf.ordered_vertices().len(), 0);
    }

    /// The oldest vertex should have the lowest alpha (near zero) and the
    /// newest should have the highest alpha (near 1.0) after quadratic fade.
    #[test]
    fn fade_increases_from_tail_to_head() {
        let cap = 10;
        let mut buf = TrailBuffer::new(cap, red());
        for i in 0..cap {
            buf.push([i as f32, 0.0, 0.0]);
        }
        let verts = buf.ordered_vertices();
        // Oldest vertex (index 0) has the lowest alpha.
        let first_alpha = verts[0].color[3];
        let last_alpha = verts[cap - 1].color[3];
        assert!(
            first_alpha < last_alpha,
            "expected first_alpha < last_alpha, got {first_alpha} vs {last_alpha}"
        );
    }

    /// When the buffer wraps around, ordered_vertices must still return samples
    /// in insertion order: oldest first.
    #[test]
    fn ring_wrap_preserves_order() {
        let cap = 4;
        let mut buf = TrailBuffer::new(cap, red());
        // Fill then overshoot by 2.
        for i in 0..6u32 {
            buf.push([i as f32, 0.0, 0.0]);
        }
        // The last 4 inserted values are [2, 3, 4, 5].
        let verts = buf.ordered_vertices();
        assert_eq!(verts.len(), cap);
        let positions: Vec<f32> = verts.iter().map(|v| v.position[0]).collect();
        assert_eq!(
            positions,
            vec![2.0, 3.0, 4.0, 5.0],
            "ring wrap order incorrect: {positions:?}"
        );
    }

    /// A zero-capacity guard: TrailBuffer::new clamps capacity to 1 to avoid
    /// divide-by-zero in ordered_vertices.
    #[test]
    fn zero_capacity_is_clamped_to_one() {
        let buf = TrailBuffer::new(0, red());
        assert_eq!(buf.capacity, 1);
    }
}
