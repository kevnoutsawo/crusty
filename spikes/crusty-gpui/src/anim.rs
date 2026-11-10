//! Time-based animation helpers.
//!
//! GPUI has no CSS-style transitions: a view re-renders when notified, so an
//! animation is just "derive the value from elapsed time, and request another
//! frame while it is still moving". Everything here is a pure function of
//! `Instant::now()`, which keeps animations interruptible for free.

use std::time::{Duration, Instant};

pub fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

pub fn ease_in_out_cubic(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// Overshoots past 1.0 before settling: gives "pop" to scale animations.
pub fn ease_out_back(t: f32) -> f32 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
}

/// Damped spring settling at 1.0; used for the slightly bouncy pill and rows.
pub fn spring(t: f32) -> f32 {
    1.0 - (-6.0 * t).exp() * (8.0 * t).cos()
}

/// Linear progress in 0..=1 of an animation that began at `start`.
/// `None` means "never started", which reads as already finished.
pub fn progress(start: Option<Instant>, dur: Duration) -> f32 {
    match start {
        Some(start) => (start.elapsed().as_secs_f32() / dur.as_secs_f32()).clamp(0.0, 1.0),
        None => 1.0,
    }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// A scalar that glides to a new target whenever it is set.
#[derive(Clone, Copy)]
pub struct Tween {
    from: f32,
    to: f32,
    start: Instant,
    dur: Duration,
    ease: fn(f32) -> f32,
}

impl Tween {
    pub fn new(value: f32, dur: Duration, ease: fn(f32) -> f32) -> Self {
        Self {
            from: value,
            to: value,
            start: Instant::now() - dur,
            dur,
            ease,
        }
    }

    /// Retarget from wherever the value currently is, so interrupting a
    /// running animation never jumps.
    pub fn set(&mut self, to: f32) {
        if (to - self.to).abs() < f32::EPSILON {
            return;
        }
        self.from = self.value();
        self.to = to;
        self.start = Instant::now();
    }

    /// Jump to `from` and animate toward `to` (used for FLIP-style offsets).
    pub fn animate_from(&mut self, from: f32, to: f32) {
        self.from = from;
        self.to = to;
        self.start = Instant::now();
    }

    /// Hold the current value for `d` before moving (staggered entrances).
    pub fn delay(&mut self, d: Duration) {
        self.start += d;
    }

    pub fn value(&self) -> f32 {
        let t = progress(Some(self.start), self.dur);
        lerp(self.from, self.to, (self.ease)(t))
    }

    pub fn target(&self) -> f32 {
        self.to
    }

    pub fn is_animating(&self) -> bool {
        self.start.elapsed() < self.dur
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easings_hit_endpoints() {
        for f in [ease_out_cubic, ease_in_out_cubic, ease_out_back] {
            assert!(f(0.0).abs() < 1e-5);
            assert!((f(1.0) - 1.0).abs() < 1e-5);
        }
        assert!((spring(1.0) - 1.0).abs() < 0.01);
    }

    #[test]
    fn tween_retargets_without_jumping() {
        let mut t = Tween::new(0.0, Duration::from_millis(200), ease_out_cubic);
        assert_eq!(t.value(), 0.0);
        t.set(10.0);
        assert!(t.is_animating());
        assert!(t.value() < 10.0);
        let before = t.value();
        t.set(-5.0);
        assert!((t.value() - before).abs() < 0.5);
        assert_eq!(t.target(), -5.0);
    }
}
