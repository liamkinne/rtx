#![no_std]
#![no_main]

mod motor;

use defmt_rtt as _;
use embassy_stm32 as hal;
use panic_probe as _;

use crate::motor::Motor;
use crate::motor::PidGains;
use core::mem::MaybeUninit;
use embassy_usb::UsbDevice;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use hal::bind_interrupts;
use hal::gpio::Level;
use hal::gpio::Output;
use hal::gpio::Speed;
use hal::i2c;
use hal::mode::Async;
use hal::peripherals::*;
use hal::time::Hertz;
use hal::timer::qei::Config;
use hal::timer::qei::Qei;
use hal::{gpio::Pull, timer::qei::QeiMode, usb::Driver};
use pwm_pca9685::Address;
use pwm_pca9685::Channel;
use pwm_pca9685::Pca9685;
use rtic_monotonics::Monotonic;
use rtic_monotonics::systick::prelude::*;
use rtic_monotonics::systick_monotonic;
use rtic_sync::arbiter::Arbiter;

pub mod pac {
    pub use embassy_stm32::pac::Interrupt as interrupt;
    pub use embassy_stm32::pac::*;
}

bind_interrupts!(struct Irqs {
    I2C2_ER => i2c::ErrorInterruptHandler<I2C2>;
    I2C2_EV => i2c::EventInterruptHandler<I2C2>;
    DMA1_CHANNEL1 => hal::dma::InterruptHandler<DMA1_CH1>;
    DMA1_CHANNEL2 => hal::dma::InterruptHandler<DMA1_CH2>;
    USB_LP => hal::usb::InterruptHandler<USB>;
});

systick_monotonic!(Mono, 10_000);
defmt::timestamp!("{=u32:tus}", Mono::now().duration_since_epoch().to_micros());

#[rtic::app(device = pac, peripherals = false)]
mod app {
    use super::*;

    type I2C = i2c::I2c<'static, Async, i2c::Master>;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led_status: Output<'static>,
        led_error: Output<'static>,
        usb_class: CdcAcmClass<'static, Driver<'static, USB>>,
        usb_device: UsbDevice<'static, Driver<'static, USB>>,
        motor_1: Motor<'static, TIM1, I2C>,
        motor_2: Motor<'static, TIM2, I2C>,
        motor_3: Motor<'static, TIM3, I2C>,
        motor_4: Motor<'static, TIM4, I2C>,
        motor_5: Motor<'static, TIM5, I2C>,
        motor_6: Motor<'static, TIM8, I2C>,
        motor_7: Motor<'static, TIM20, I2C>,
        pwm_oe: Output<'static>,
    }

    #[init(local = [
        pca9685: MaybeUninit<Arbiter<Pca9685<I2C>>> = MaybeUninit::uninit(),
        usb_config_descriptor: [u8; 256] = [0; 256],
        usb_bos_descriptor: [u8; 256] = [0; 256],
        usb_control_buf: [u8; 64] = [0; 64],
        usb_state: State<'static> = State::new(),
    ])]
    fn init(cx: init::Context) -> (Shared, Local) {
        let mut config = hal::Config::default();
        {
            use embassy_stm32::rcc::*;
            config.rcc.hse = Some(Hse {
                freq: Hertz::mhz(24),
                mode: HseMode::Oscillator,
            });
            config.rcc.pll = Some(Pll {
                source: PllSource::HSE,
                prediv: PllPreDiv::DIV6,
                mul: PllMul::MUL80,
                divp: None,
                divq: Some(PllQDiv::DIV4), // 80 MHz for fdcan
                divr: Some(PllRDiv::DIV2), // Main system clock at 160 MHz
            });
            config.rcc.mux.fdcansel = mux::Fdcansel::PLL1_Q;
            config.rcc.mux.adc12sel = mux::Adcsel::SYS;
            config.rcc.mux.adc345sel = mux::Adcsel::SYS;
            config.rcc.sys = Sysclk::PLL1_R;
            config.rcc.mux.clk48sel = mux::Clk48sel::HSI48;
        }
        let p = hal::init(config);

        Mono::start(cx.core.SYST, 160_000_000);

        let led_status = Output::new(p.PE15, Level::Low, Speed::Low);
        let led_error = Output::new(p.PE14, Level::Low, Speed::Low);

        let config = Config {
            ch1_pull: Pull::Up,
            ch2_pull: Pull::Up,
            mode: QeiMode::Mode1,
            ..Default::default()
        };
        let qei1 = Qei::new(p.TIM1, p.PE9, p.PE11, config);
        let qei2 = Qei::new(p.TIM2, p.PD3, p.PD4, config);
        let qei3 = Qei::new(p.TIM3, p.PA6, p.PA7, config);
        let qei4 = Qei::new(p.TIM4, p.PD12, p.PD13, config);
        let qei5 = Qei::new(p.TIM5, p.PA0, p.PA1, config);
        let qei6 = Qei::new(p.TIM8, p.PC6, p.PC7, config);
        let qei7 = Qei::new(p.TIM20, p.PE2, p.PE3, config);

        let i2c2 = hal::i2c::I2c::new(p.I2C2, p.PA9, p.PA8, p.DMA1_CH1, p.DMA1_CH2, Irqs, {
            let mut cfg = hal::i2c::Config::default();
            cfg.scl_pullup = true;
            cfg.sda_pullup = true;
            cfg.gpio_speed = Speed::Low;
            cfg.frequency = Hertz::khz(100);
            cfg
        });
        let pca9685 = pwm_pca9685::Pca9685::new(i2c2, Address::default()).unwrap();
        let pwm = cx.local.pca9685.write(Arbiter::new(pca9685));
        let pwm_oe = Output::new(p.PE1, Level::Low, Speed::Low);

        let motor_1 = Motor::new(qei1, pwm, Channel::C13, Channel::C12);
        let motor_2 = Motor::new(qei2, pwm, Channel::C10, Channel::C11);
        let motor_3 = Motor::new(qei3, pwm, Channel::C8, Channel::C9);
        let motor_4 = Motor::new(qei4, pwm, Channel::C6, Channel::C7);
        let motor_5 = Motor::new(qei5, pwm, Channel::C4, Channel::C5);
        let motor_6 = Motor::new(qei6, pwm, Channel::C2, Channel::C3);
        let motor_7 = Motor::new(qei7, pwm, Channel::C0, Channel::C1);

        let usb = hal::usb::Driver::new(p.USB, Irqs, p.PA12, p.PA11);
        let mut config = embassy_usb::Config::new(0x0483, 0x5740);
        config.manufacturer = Some("Universal Machine Intelligence");
        config.product = Some("UMI RTX Driver");
        config.self_powered = true;
        let mut builder = embassy_usb::Builder::new(
            usb,
            config,
            cx.local.usb_config_descriptor,
            cx.local.usb_bos_descriptor,
            &mut [], // no msos descriptors
            cx.local.usb_control_buf,
        );
        let usb_class = CdcAcmClass::new(&mut builder, cx.local.usb_state, 64);
        let usb_device = builder.build();

        usb::spawn().unwrap();

        (
            Shared {},
            Local {
                led_status,
                led_error,
                usb_class,
                usb_device,
                motor_1,
                motor_2,
                motor_3,
                motor_4,
                motor_5,
                motor_6,
                motor_7,
                pwm_oe,
            },
        )
    }

    /// Blink the status led to show activity.
    #[task(local = [led_status])]
    async fn activity(cx: activity::Context) {
        cx.local.led_status.set_high();
        Mono::delay(10.millis()).await;
        cx.local.led_status.set_low();
        Mono::delay(10.millis()).await;
    }

    /// Blink the error led to show error activity.
    #[task(local = [led_error])]
    async fn error(cx: error::Context) {
        cx.local.led_error.set_high();
        Mono::delay(80.millis()).await;
        cx.local.led_error.set_low();
        Mono::delay(20.millis()).await;
    }

    #[task(local = [usb_device])]
    async fn usb(cx: usb::Context) {
        cx.local.usb_device.run().await
    }
}
