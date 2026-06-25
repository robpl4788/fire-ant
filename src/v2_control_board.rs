use crate::drivers::{bldc, rgb_led};

use embassy_rp::Peri;
use embassy_rp::peripherals::{
    PIN_10, PIN_11, PIN_12, PIN_13, PIN_14, PIN_15, PIN_16, PIN_17, PIN_18, PWM_SLICE0, PWM_SLICE1,
    PWM_SLICE5, PWM_SLICE6, PWM_SLICE7,
};
use embassy_rp::pwm::{Config, Pwm, PwmOutput};

// Type aliases for PWM outputs
pub type RGBRed = PwmOutput<'static>;
pub type RGBGreen = PwmOutput<'static>;
pub type RGBBlue = PwmOutput<'static>;

pub type RGBLed = crate::drivers::rgb_led::RGBLed<RGBRed, RGBGreen, RGBBlue>;

pub type BLDCLowA = PwmOutput<'static>;
pub type BLDCLowB = PwmOutput<'static>;
pub type BLDCLowC = PwmOutput<'static>;

pub type BLDCHighA = PwmOutput<'static>;
pub type BLDCHighB = PwmOutput<'static>;
pub type BLDCHighC = PwmOutput<'static>;

pub type BLDC =
    crate::drivers::bldc::BLDC<BLDCLowA, BLDCLowB, BLDCLowC, BLDCHighA, BLDCHighB, BLDCHighC>;

// PWM frequency configurations
const LED_PWM_TOP: u16 = 65535;
const BLDC_PWM_TOP: u16 = 7499;

/// Main control board for Fire Ant
pub struct FireAntBoard {
    pub rgb: RGBLed,
    pub bldc: BLDC,
}

/// Builder for FireAntBoard to avoid parameter explosion
pub struct FireAntBoardBuilder {
    rgb_pwm: Option<(
        Peri<'static, PWM_SLICE0>,
        Peri<'static, PIN_16>,
        Peri<'static, PIN_17>,
        Peri<'static, PWM_SLICE1>,
        Peri<'static, PIN_18>,
    )>,
    bldc_pwm: Option<(
        Peri<'static, PWM_SLICE5>,
        Peri<'static, PIN_10>,
        Peri<'static, PIN_11>,
        Peri<'static, PWM_SLICE6>,
        Peri<'static, PIN_12>,
        Peri<'static, PIN_13>,
        Peri<'static, PWM_SLICE7>,
        Peri<'static, PIN_14>,
        Peri<'static, PIN_15>,
    )>,
}

impl FireAntBoardBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            rgb_pwm: None,
            bldc_pwm: None,
        }
    }

    /// Set RGB channels
    pub fn with_rgb_pwm(
        mut self,
        pwm_rg: Peri<'static, PWM_SLICE0>,
        red: Peri<'static, PIN_16>,
        green: Peri<'static, PIN_17>,
        pwm_b: Peri<'static, PWM_SLICE1>,
        blue: Peri<'static, PIN_18>,
    ) -> Self {
        self.rgb_pwm = Some((pwm_rg, red, green, pwm_b, blue));
        self
    }

    /// Set all BLDC phase pins
    pub fn with_bldc_phases(
        mut self,
        pwm_lowc_lowb: Peri<'static, PWM_SLICE5>,
        pin_lowc: Peri<'static, PIN_10>,
        pin_lowb: Peri<'static, PIN_11>,
        pwm_lowa_highc: Peri<'static, PWM_SLICE6>,
        pin_lowa: Peri<'static, PIN_12>,
        pin_highc: Peri<'static, PIN_13>,
        pwm_highb_higha: Peri<'static, PWM_SLICE7>,
        pin_highb: Peri<'static, PIN_14>,
        pin_higha: Peri<'static, PIN_15>,
    ) -> Self {
        self.bldc_pwm = Some((
            pwm_lowc_lowb,
            pin_lowc,
            pin_lowb,
            pwm_lowa_highc,
            pin_lowa,
            pin_highc,
            pwm_highb_higha,
            pin_highb,
            pin_higha,
        ));
        self
    }

    /// Build the FireAntBoard
    pub fn build(mut self) -> FireAntBoard {
        let rgb = self.build_rgb();
        let bldc = self.build_bldc();
        FireAntBoard { rgb, bldc }
    }

    fn build_rgb(&mut self) -> RGBLed {
        let (pwm_rg, red_pin, green_pin, pwm_b, blue_pin) =
            self.rgb_pwm.take().expect("RGB PWM not configured");

        let mut led_cfg = Config::default();
        led_cfg.top = LED_PWM_TOP;

        let red_green_pwm = Pwm::new_output_ab(pwm_rg, red_pin, green_pin, led_cfg.clone());
        let blue_pwm = Pwm::new_output_a(pwm_b, blue_pin, led_cfg);

        let (red_ch, green_ch) = red_green_pwm.split();
        let (blue_ch, _) = blue_pwm.split();

        rgb_led::RGBLed::new(
            red_ch.expect("Red channel split failed"),
            green_ch.expect("Green channel split failed"),
            blue_ch.expect("Blue channel split failed"),
        )
    }

    fn build_bldc(&mut self) -> BLDC {
        let (
            pwm_lowc_lowb,
            pin_lowc,
            pin_lowb,
            pwm_lowa_highc,
            pin_lowa,
            pin_highc,
            pwm_highb_higha,
            pin_highb,
            pin_higha,
        ) = self.bldc_pwm.take().expect("BLDC PWM not configured");

        let mut bldc_cfg = Config::default();
        bldc_cfg.top = BLDC_PWM_TOP;

        let lowc_lowb_pwm = Pwm::new_output_ab(pwm_lowc_lowb, pin_lowc, pin_lowb, bldc_cfg.clone());
        let lowa_highc_pwm =
            Pwm::new_output_ab(pwm_lowa_highc, pin_lowa, pin_highc, bldc_cfg.clone());
        let highb_higha_pwm = Pwm::new_output_ab(pwm_highb_higha, pin_highb, pin_higha, bldc_cfg);

        let (lowc_ch, lowb_ch) = lowc_lowb_pwm.split();
        let (lowa_ch, highc_ch) = lowa_highc_pwm.split();
        let (highb_ch, higha_ch) = highb_higha_pwm.split();

        bldc::BLDC::new(
            lowa_ch.expect("Phase A low split failed"),
            lowb_ch.expect("Phase B low split failed"),
            lowc_ch.expect("Phase C low split failed"),
            higha_ch.expect("Phase A high split failed"),
            highb_ch.expect("Phase B high split failed"),
            highc_ch.expect("Phase C high split failed"),
        )
    }
}
