use crate::drivers::{bldc, rgb_led};

use embassy_rp::Peri;
use embassy_rp::peripherals::{
    PIN_10, PIN_11, PIN_12, PIN_13, PIN_14, PIN_15, PIN_16, PIN_17, PIN_18, PWM_SLICE0, PWM_SLICE1,
    PWM_SLICE5, PWM_SLICE6, PWM_SLICE7,
};
use embassy_rp::pwm::{Config, Pwm, PwmOutput};

pub type RGBRed = PwmOutput<'static>;
pub type RGBGreen = PwmOutput<'static>;
pub type RGBBlue = PwmOutput<'static>;

pub type RGBLed = crate::drivers::rgb_led::RGBLed<RGBRed, RGBGreen, RGBBlue>;

pub type BLDCLowC = PwmOutput<'static>;
pub type BLDCLowB = PwmOutput<'static>;
pub type BLDCLowA = PwmOutput<'static>;

pub type BLDCHighC = PwmOutput<'static>;
pub type BLDCHighB = PwmOutput<'static>;
pub type BLDCHighA = PwmOutput<'static>;

pub type BLDC =
    crate::drivers::bldc::BLDC<BLDCLowA, BLDCLowB, BLDCLowC, BLDCHighA, BLDCHighB, BLDCHighC>;

const LED_TOP: u16 = 65535;
const BLDC_TOP: u16 = 7499;

pub struct FireAntBoard {
    pub rgb: RGBLed,
    pub bldc: BLDC,
}

impl FireAntBoard {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pwm0: Peri<'static, PWM_SLICE0>,
        gpio16: Peri<'static, PIN_16>,
        gpio17: Peri<'static, PIN_17>,
        pwm1: Peri<'static, PWM_SLICE1>,
        gpio18: Peri<'static, PIN_18>,
        pwm5: Peri<'static, PWM_SLICE5>,
        gpio10: Peri<'static, PIN_10>,
        gpio11: Peri<'static, PIN_11>,
        pwm6: Peri<'static, PWM_SLICE6>,
        gpio12: Peri<'static, PIN_12>,
        gpio13: Peri<'static, PIN_13>,
        pwm7: Peri<'static, PWM_SLICE7>,
        gpio14: Peri<'static, PIN_14>,
        gpio15: Peri<'static, PIN_15>,
    ) -> Self {
        let mut led_config = Config::default();
        led_config.top = LED_TOP;

        let red_green_pwm = Pwm::new_output_ab(pwm0, gpio16, gpio17, led_config.clone());
        let blue_pwm = Pwm::new_output_a(pwm1, gpio18, led_config);

        let (red_channel, green_channel) = red_green_pwm.split();
        let (blue_channel, _) = blue_pwm.split();

        let rgb = rgb_led::RGBLed::new(
            red_channel.unwrap(),
            green_channel.unwrap(),
            blue_channel.unwrap(),
        );

        let mut bldc_config = Config::default();
        bldc_config.top = BLDC_TOP;

        let lowc_lowb_pwm = Pwm::new_output_ab(pwm5, gpio10, gpio11, bldc_config.clone());
        let lowa_highc_pwm = Pwm::new_output_ab(pwm6, gpio12, gpio13, bldc_config.clone());
        let highb_higha_pwm = Pwm::new_output_ab(pwm7, gpio14, gpio15, bldc_config);

        let (lowc_channel, lowb_channel) = lowc_lowb_pwm.split();
        let (lowa_channel, highc_channel) = lowa_highc_pwm.split();
        let (highb_channel, higha_channel) = highb_higha_pwm.split();

        let bldc = bldc::BLDC::new(
            lowa_channel.unwrap(),
            lowb_channel.unwrap(),
            lowc_channel.unwrap(),
            higha_channel.unwrap(),
            highb_channel.unwrap(),
            highc_channel.unwrap(),
        );

        Self { rgb, bldc }
    }
}
