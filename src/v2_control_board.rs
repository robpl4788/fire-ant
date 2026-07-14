use embassy_rp::Peri;
use embassy_rp::config::Config as RpConfig;
use embassy_rp::peripherals;
use embassy_rp::pwm::{self, Pwm};
use embassy_rp::{Peripherals, clocks::ClockConfig};

use crate::drivers::rgb_led::{self, RGBLed};

type RGBLedType = RGBLed<pwm::PwmOutput<'static>, pwm::PwmOutput<'static>, pwm::PwmOutput<'static>>;

const LED_PWM_TOP: u16 = 65535;

pub struct V2ControlBoard {
    rgb: Option<RGBLedType>,
    pwm_rg: Option<Peri<'static, peripherals::PWM_SLICE0>>,
    pwm_b: Option<Peri<'static, peripherals::PWM_SLICE1>>,
    red_pin: Option<Peri<'static, peripherals::PIN_16>>,
    green_pin: Option<Peri<'static, peripherals::PIN_17>>,
    blue_pin: Option<Peri<'static, peripherals::PIN_18>>,
}

impl V2ControlBoard {
    pub fn new() -> Self {
        // Initialize hardware
        let mut config = RpConfig::default();
        config.clocks = ClockConfig::crystal(12_000_000);
        let peripherals = embassy_rp::init(config);
        let mut result = V2ControlBoard {
            rgb: None,
            red_pin: Some(peripherals.PIN_16),
            green_pin: Some(peripherals.PIN_17),
            blue_pin: Some(peripherals.PIN_18),
            pwm_rg: Some(peripherals.PWM_SLICE0),
            pwm_b: Some(peripherals.PWM_SLICE1),
        };

        result.build_rgb();

        result
    }

    fn build_rgb(&mut self) {
        let mut led_cfg = pwm::Config::default();
        led_cfg.top = LED_PWM_TOP;
        led_cfg.enable = true;

        let red_green_pwm = Pwm::new_output_ab(
            self.pwm_rg.take().expect("Already Built RGB"),
            self.red_pin.take().expect("Already Built RGB"),
            self.green_pin.take().expect("Already Built RGB"),
            led_cfg.clone(),
        );
        let blue_pwm = Pwm::new_output_a(
            self.pwm_b.take().expect("Already Built RGB"),
            self.blue_pin.take().expect("Already Built RGB"),
            led_cfg,
        );

        let (red_ch, green_ch) = red_green_pwm.split();
        let (blue_ch, _) = blue_pwm.split();

        self.rgb = Some(rgb_led::RGBLed::new(
            red_ch.expect("Red Channel Failed"),
            green_ch.expect("Green Channel Failed"),
            blue_ch.expect("Blue Channel Failed"),
        ));
    }

    pub fn take_rgb(&mut self) -> RGBLedType {
        let result = self.rgb.take();
        result.expect("Already took RGB Led")
    }
}
