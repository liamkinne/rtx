#![cfg_attr(not(test), no_std)]

pub mod pid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub zed: f32,
    pub shoulder: f32,
    pub elbow: f32,
    pub yaw: f32,
    pub wrist_pitch: f32,
    pub wrist_roll: f32,
    pub grip: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Zed,
    Shoulder,
    Elbow,
    Yaw,
    WristPitch,
    WristRoll,
    Grip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisKind {
    Linear,
    Rotary,
    Orbital,
}

pub struct AxisLimits {
    /// Maximum velocity. units/sec, must be > 0
    pub max_velocity: f32,
    /// Maximum acceleration. units/sec^2, must be > 0
    pub max_acceleration: f32,
    /// Maximum jerk. units/sec^3, None = jerk unconstrained (trapezoidal profile)
    pub max_jerk: Option<f32>,
}
