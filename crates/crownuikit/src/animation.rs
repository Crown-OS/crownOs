//! Spring-driven scalar interpolation. A single critically-damped harmonic
//! oscillator, integrated with fixed sub-steps so the integrator stays stable
//! independent of the frame rate the compositor wakes us at.
//!
//! Each animated value carries its own position + velocity, and a target. Step
//! the value with [`Spring::step`] when the frame ticks; check
//! [`Spring::at_rest`] to know when to stop requesting frames.
//!
//! Pick a "feel" per widget by choosing a [`SpringProfile`]. The default
//! [`SpringProfile::SNAPPY`] settles in roughly 200 ms with no overshoot; the
//! softer [`SpringProfile::SMOOTH`] is what dropdown / picker-style widgets use
//! when a value slides across a distance and needs to look unhurried.

use std::time::Instant;

/// Fixed integrator sub-step.
const SUBSTEP: f32 = 1.0 / 240.0;
/// Largest dt the integrator will accept in one tick.
const MAX_DT: f32 = 1.0 / 30.0;
/// Settle thresholds.
const EPSILON_POS: f32 = 0.0005;
const EPSILON_VEL: f32 = 0.01;

/// Named stiffness/damping pair. `damping ≈ 2 * sqrt(stiffness)` keeps the
/// response critically damped — no overshoot, gentle ease-out.
#[derive(Debug, Clone, Copy)]
pub struct SpringProfile {
    pub stiffness: f32,
    pub damping: f32,
}

impl SpringProfile {
    /// Fast settle (~200 ms). Default for toggles and sliders — anything that
    /// tracks the pointer or reacts to a discrete tap.
    pub const SNAPPY: Self = Self {
        stiffness: 320.0,
        damping: 35.78,
    };
    /// Softer, wider-arc settle (~350 ms). Good for values that visibly slide
    /// across the widget — dropdown panels, hover highlights that traverse
    /// rows, checkmark position between selection changes.
    pub const SMOOTH: Self = Self {
        stiffness: 180.0,
        damping: 26.83,
    };
}

#[derive(Debug, Clone, Copy)]
pub struct Spring {
    pub position: f32,
    pub velocity: f32,
    pub target: f32,
    pub profile: SpringProfile,
}

impl Spring {
    pub const fn new(value: f32) -> Self {
        Self::with_profile(value, SpringProfile::SNAPPY)
    }

    pub const fn with_profile(value: f32, profile: SpringProfile) -> Self {
        Self {
            position: value,
            velocity: 0.0,
            target: value,
            profile,
        }
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Retarget while injecting a starting velocity — useful when the caller
    /// wants the spring to be *visibly* moving on the very next frame instead
    /// of ramping up from rest. Critically-damped springs otherwise cover
    /// only a few percent of the remaining distance in the first frame,
    /// which reads as "stuck" for close/dismiss animations.
    pub fn set_target_with_velocity(&mut self, target: f32, velocity: f32) {
        self.target = target;
        self.velocity = velocity;
    }

    /// Integrate the spring forward by `dt` seconds (clamped to MAX_DT).
    pub fn step(&mut self, dt: f32) {
        let mut remaining = dt.min(MAX_DT);
        while remaining > 0.0 {
            let h = remaining.min(SUBSTEP);
            let accel = -self.profile.stiffness * (self.position - self.target)
                - self.profile.damping * self.velocity;
            self.velocity += accel * h;
            self.position += self.velocity * h;
            remaining -= h;
        }
    }

    pub fn at_rest(&self) -> bool {
        (self.position - self.target).abs() < EPSILON_POS && self.velocity.abs() < EPSILON_VEL
    }

    #[allow(dead_code)]
    pub fn snap_to_target(&mut self) {
        self.position = self.target;
        self.velocity = 0.0;
    }
}

/// Wall-clock dt source for animation loops. `tick` returns the delta since
/// the previous call (or a 60-Hz frame on first call), clamped to MAX_DT.
pub struct Clock {
    last: Option<Instant>,
}

impl Default for Clock {
    fn default() -> Self {
        Self { last: None }
    }
}

impl Clock {
    pub const fn new() -> Self {
        Self { last: None }
    }

    pub fn reset(&mut self) {
        self.last = None;
    }

    pub fn tick(&mut self) -> f32 {
        let now = Instant::now();
        let dt = self
            .last
            .map(|t| now.duration_since(t).as_secs_f32())
            .unwrap_or(1.0 / 60.0)
            .min(MAX_DT);
        self.last = Some(now);
        dt
    }
}
