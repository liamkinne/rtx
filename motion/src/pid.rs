/// Tunable gains for the position PID control loop.
#[derive(Debug, Clone, Copy)]
pub struct Gains {
    /// Proportional gain.
    pub kp: f32,
    /// Integral gain.
    pub ki: f32,
    /// Derivative gain.
    pub kd: f32,
}

/// A simple PID controller operating on `f32` error values.
#[derive(Debug)]
pub struct Pid {
    gains: Gains,
    integral: f32,
    prev_error: f32,
}

impl Pid {
    pub fn new(gains: Gains) -> Self {
        Self {
            gains,
            integral: 0.0,
            prev_error: 0.0,
        }
    }

    /// Feeds a new error value (and the elapsed time in seconds since the
    /// last update) into the controller, returning the control output.
    pub fn update(&mut self, error: f32, dt: f32) -> f32 {
        self.integral += error * dt;
        let derivative = if dt > 0.0 {
            (error - self.prev_error) / dt
        } else {
            0.0
        };
        self.prev_error = error;

        self.gains.kp * error + self.gains.ki * self.integral + self.gains.kd * derivative
    }
}
