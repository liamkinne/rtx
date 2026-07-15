use crate::Mono;
use embassy_stm32 as hal;
use embedded_hal_async::i2c::I2c as AsyncI2c;
use hal::timer::GeneralInstance4Channel;
use hal::timer::qei::Qei;
use pwm_pca9685::Channel;
use pwm_pca9685::Pca9685;
use rtic_monotonics::Monotonic;
use rtic_monotonics::systick::prelude::*;
use rtic_sync::arbiter::Arbiter;

/// Tunable gains for the position PID control loop.
#[derive(Clone, Copy, Debug)]
pub struct PidGains {
    /// Proportional gain.
    pub kp: f32,
    /// Integral gain.
    pub ki: f32,
    /// Derivative gain.
    pub kd: f32,
}

/// A simple PID controller operating on `f32` error values.
struct Pid {
    gains: PidGains,
    integral: f32,
    prev_error: f32,
}

impl Pid {
    fn new(gains: PidGains) -> Self {
        Self {
            gains,
            integral: 0.0,
            prev_error: 0.0,
        }
    }

    /// Feeds a new error value (and the elapsed time in seconds since the
    /// last update) into the controller, returning the control output.
    fn update(&mut self, error: f32, dt_secs: f32) -> f32 {
        self.integral += error * dt_secs;
        let derivative = if dt_secs > 0.0 {
            (error - self.prev_error) / dt_secs
        } else {
            0.0
        };
        self.prev_error = error;

        self.gains.kp * error + self.gains.ki * self.integral + self.gains.kd * derivative
    }
}

pub struct Motor<'d, T: GeneralInstance4Channel, I2C: AsyncI2c> {
    qei: Qei<'d, T>,
    pwm: &'d Arbiter<Pca9685<I2C>>,
    ch_a: Channel,
    ch_b: Channel,
}

impl<'d, T: GeneralInstance4Channel, I2C: AsyncI2c> Motor<'d, T, I2C> {
    /// The period between control loop iterations, in milliseconds.
    const CONTROL_PERIOD_MS: u32 = 10;

    /// Creates a new motor instance.
    pub fn new(
        qei: Qei<'d, T>,
        pwm: &'d Arbiter<Pca9685<I2C>>,
        ch_a: Channel,
        ch_b: Channel,
    ) -> Self {
        Self {
            qei,
            pwm,
            ch_a,
            ch_b,
        }
    }

    pub async fn setup(&self) {
        let mut pwm = self.pwm.access().await;
        pwm.set_prescale(100).await.unwrap();
        pwm.enable().await.unwrap();
        pwm.set_channel_on(self.ch_a, 0).await.unwrap();
        pwm.set_channel_on(self.ch_b, 0).await.unwrap();
    }

    /// Sets the motor speed.
    ///
    /// `speed` is clamped to `[-1.0, 1.0]`, where positive values drive the
    /// motor forward (channel A) and negative values drive it in reverse
    /// (channel B).
    pub async fn set(&self, speed: f32) {
        const MAX_DUTY: f32 = 4095.0;

        let speed = speed.clamp(-1.0, 1.0);
        let duty = (speed.abs() * MAX_DUTY) as u16;
        let (duty_a, duty_b) = if speed >= 0.0 { (duty, 0) } else { (0, duty) };

        let mut pwm = self.pwm.access().await;
        pwm.set_channel_off(self.ch_a, duty_a).await.unwrap();
        pwm.set_channel_off(self.ch_b, duty_b).await.unwrap();
    }

    /// Returns the current, unitless quadrature encoder count.
    pub fn position(&self) -> u16 {
        self.qei.count()
    }

    /// Resets the quadrature encoder count back to zero.
    pub fn reset_position(&mut self) {
        self.qei.reset();
    }

    /// Drives the motor to `setpoint` (an absolute encoder count) using a
    /// PID control loop fed by the quadrature encoder.
    ///
    /// `gains` configures the PID controller, `max_speed` caps the magnitude
    /// of the speed passed to [`Motor::set`] (clamped to `[0.0, 1.0]`), and
    /// `tolerance` is the number of encoder counts within which the
    /// setpoint is considered reached.
    ///
    /// `invert` flips the sign of the control output before it's applied to
    /// [`Motor::set`]. This is useful when the encoder's counting direction
    /// is opposite to the motor's wiring polarity for a given motor, so a
    /// positive PID output would otherwise drive the position further from
    /// (rather than towards) the setpoint.
    ///
    /// This runs until the motor settles within `tolerance` of `setpoint`,
    /// at which point the motor is stopped and the function returns.
    pub async fn run_to_position(
        &self,
        setpoint: u16,
        gains: PidGains,
        max_speed: f32,
        tolerance: u16,
    ) {
        let dt_secs = Self::CONTROL_PERIOD_MS as f32 / 1000.0;

        let max_speed = max_speed.clamp(0.0, 1.0);
        let mut pid = Pid::new(gains);

        loop {
            let position = self.qei.count();
            // The QEI counter is a free-running 16-bit counter that wraps
            // around (both forwards and backwards). Computing a plain
            // `setpoint - position` difference breaks down across a wrap
            // boundary, producing a huge error and driving the motor the
            // long way around. Instead, take the wrapping difference in
            // `u16` space and reinterpret it as a signed `i16`, which
            // yields the shortest signed distance (in `[-32768, 32767]`)
            // from `position` to `setpoint` around the 16-bit circle.
            let error = setpoint.wrapping_sub(position) as i16 as i32;

            if error.unsigned_abs() <= tolerance as u32 {
                break;
            }

            let output = pid.update(error as f32, dt_secs);
            self.set(output.clamp(-max_speed, max_speed)).await;

            Mono::delay(Self::CONTROL_PERIOD_MS.millis()).await;
        }

        self.set(0.0).await;
    }
}
