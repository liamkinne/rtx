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

pub struct Motor<'d, T: GeneralInstance4Channel, I2C: AsyncI2c> {
    qei: Qei<'d, T>,
    pwm: &'d Arbiter<Pca9685<I2C>>,
    channels: (Channel, Channel),
}

impl<'d, T: GeneralInstance4Channel, I2C: AsyncI2c> Motor<'d, T, I2C> {
    /// The period between control loop iterations, in milliseconds.
    const CONTROL_PERIOD_MS: u32 = 2;
    /// Setpoint tolerance.
    const TOLERANCE: u16 = 5;

    /// Creates a new motor instance.
    pub fn new(
        qei: Qei<'d, T>,
        pwm: &'d Arbiter<Pca9685<I2C>>,
        channels: (Channel, Channel),
    ) -> Self {
        Self { qei, pwm, channels }
    }

    /// Setup the PWM outputs.
    ///
    /// Do this before making any movements.
    pub async fn setup(&self) {
        let mut pwm = self.pwm.access().await;
        pwm.set_prescale(100).await.unwrap();
        pwm.enable().await.unwrap();
        pwm.set_channel_on(self.channels.0, 0).await.unwrap();
        pwm.set_channel_off(self.channels.0, 0).await.unwrap();
        pwm.set_channel_on(self.channels.1, 0).await.unwrap();
        pwm.set_channel_off(self.channels.1, 0).await.unwrap();
    }

    /// Put the driver into the braking state.
    pub async fn brake(&self) {
        let mut pwm = self.pwm.access().await;
        pwm.set_channel_on(self.channels.0, 4095).await.unwrap();
        pwm.set_channel_off(self.channels.0, 4095).await.unwrap();
        pwm.set_channel_on(self.channels.1, 4095).await.unwrap();
        pwm.set_channel_off(self.channels.1, 4095).await.unwrap();
    }

    /// Sets the motor speed.
    ///
    /// `speed` is clamped to `[-1.0, 1.0]`.
    pub async fn set(&self, speed: f32) {
        const MAX_DUTY: f32 = 4095.0;

        let speed = speed.clamp(-1.0, 1.0);
        let duty = (speed.abs() * MAX_DUTY) as u16;
        let (duty_a, duty_b) = if speed >= 0.0 { (duty, 0) } else { (0, duty) };

        let mut pwm = self.pwm.access().await;
        pwm.set_channel_on(self.channels.0, 0).await.unwrap();
        pwm.set_channel_off(self.channels.0, duty_a).await.unwrap();
        pwm.set_channel_on(self.channels.1, 0).await.unwrap();
        pwm.set_channel_off(self.channels.1, duty_b).await.unwrap();
    }

    /// Returns the current, unitless quadrature encoder count.
    pub fn position(&self) -> i16 {
        // as signed to be centred around zero.
        self.qei.count() as i16
    }

    /// Resets the quadrature encoder count back to zero.
    pub fn reset_position(&mut self) {
        self.qei.reset();
    }

    /// Drives the motor to setpoint absolute encoder count.
    pub async fn run_to_position(&self, setpoint: i16, gains: motion::pid::Gains, max_speed: f32) {
        let dt_secs = Self::CONTROL_PERIOD_MS as f32 / 1000.0;

        let max_speed = max_speed.clamp(0.0, 1.0);
        let mut pid = motion::pid::Pid::new(gains);

        loop {
            let error = setpoint.wrapping_sub(self.position());
            if error.unsigned_abs() <= Self::TOLERANCE {
                break;
            }
            let output = pid.update(error as f32, dt_secs);
            self.set(output.clamp(-max_speed, max_speed)).await;
            Mono::delay(Self::CONTROL_PERIOD_MS.millis()).await;
        }

        self.brake().await;
    }
}
