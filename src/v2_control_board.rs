use embassy_rp::Peri;
use embassy_rp::config::Config as RpConfig;
use embassy_rp::peripherals;
use embassy_rp::pwm::{self, Pwm};
use embassy_rp::{Peripherals, clocks::ClockConfig};

use crate::drivers;
use crate::drivers::bldc::BLDC;
use crate::drivers::rgb_led::{self, RGBLed};

type RGBLedType = RGBLed<pwm::PwmOutput<'static>, pwm::PwmOutput<'static>, pwm::PwmOutput<'static>>;
type BLDCType = BLDC<
    pwm::PwmOutput<'static>,
    pwm::PwmOutput<'static>,
    pwm::PwmOutput<'static>,
    pwm::PwmOutput<'static>,
    pwm::PwmOutput<'static>,
    pwm::PwmOutput<'static>,
>;

const LED_PWM_TOP: u16 = 65535;
const BLDC_PWM_TOP: u16 = 7499;

pub struct V2ControlBoard {
    rgb: Option<RGBLedType>,
    bldc: Option<BLDCType>,

    pwm_slice_red_green: Option<Peri<'static, peripherals::PWM_SLICE0>>,
    pwm_slice_blue: Option<Peri<'static, peripherals::PWM_SLICE1>>,

    pwm_slice_lowc_lowb: Option<Peri<'static, peripherals::PWM_SLICE5>>,
    pwm_slice_lowa_highc: Option<Peri<'static, peripherals::PWM_SLICE6>>,
    pwm_slice_highb_highc: Option<Peri<'static, peripherals::PWM_SLICE7>>,

    red_pin: Option<Peri<'static, peripherals::PIN_16>>,
    green_pin: Option<Peri<'static, peripherals::PIN_17>>,
    blue_pin: Option<Peri<'static, peripherals::PIN_18>>,

    lowa_pin: Option<Peri<'static, peripherals::PIN_12>>,
    lowb_pin: Option<Peri<'static, peripherals::PIN_11>>,
    lowc_pin: Option<Peri<'static, peripherals::PIN_10>>,
    higha_pin: Option<Peri<'static, peripherals::PIN_15>>,
    highb_pin: Option<Peri<'static, peripherals::PIN_14>>,
    highc_pin: Option<Peri<'static, peripherals::PIN_13>>,
}

impl V2ControlBoard {
    pub fn new() -> Self {
        // Initialize hardware
        let mut config = RpConfig::default();
        config.clocks = ClockConfig::crystal(12_000_000);
        let peripherals = embassy_rp::init(config);
        let mut result = V2ControlBoard {
            rgb: None,
            bldc: None,

            red_pin: Some(peripherals.PIN_16),
            green_pin: Some(peripherals.PIN_17),
            blue_pin: Some(peripherals.PIN_18),

            pwm_slice_red_green: Some(peripherals.PWM_SLICE0),
            pwm_slice_blue: Some(peripherals.PWM_SLICE1),

            pwm_slice_lowc_lowb: Some(peripherals.PWM_SLICE5),
            pwm_slice_lowa_highc: Some(peripherals.PWM_SLICE6),
            pwm_slice_highb_highc: Some(peripherals.PWM_SLICE7),

            lowa_pin: Some(peripherals.PIN_12),
            lowb_pin: Some(peripherals.PIN_11),
            lowc_pin: Some(peripherals.PIN_10),
            higha_pin: Some(peripherals.PIN_15),
            highb_pin: Some(peripherals.PIN_14),
            highc_pin: Some(peripherals.PIN_13),
        };

        result.build_rgb();
        result.build_bldc();

        result.enable_pwm();

        result
    }

    fn enable_pwm(&mut self) {
        embassy_rp::pac::PWM.en().write(|w| {
            w.set_ch0(true);
            w.set_ch1(true);
            w.set_ch4(true);
            w.set_ch5(true);
            w.set_ch6(true);
            w.set_ch7(true);
        });
    }

    fn build_bldc(&mut self) {
        let mut bldc_pwm_config = pwm::Config::default();
        bldc_pwm_config.top = BLDC_PWM_TOP;
        bldc_pwm_config.enable = false;

        let mut lowc_lowb_config = bldc_pwm_config.clone();
        lowc_lowb_config.invert_a = true;
        lowc_lowb_config.invert_b = true;

        let mut lowa_highc_config = bldc_pwm_config.clone();
        lowa_highc_config.invert_a = true;

        let highb_higha_config = bldc_pwm_config.clone();

        let lowc_lowb_pwm = Pwm::new_output_ab(
            self.pwm_slice_lowc_lowb.take().expect("Already built bldc"),
            self.lowc_pin.take().expect("Already built bldc"),
            self.lowb_pin.take().expect("Already built bldc"),
            lowc_lowb_config,
        );
        let lowa_highc_pwm = Pwm::new_output_ab(
            self.pwm_slice_lowa_highc
                .take()
                .expect("Already built bldc"),
            self.lowa_pin.take().expect("Already built bldc"),
            self.highc_pin.take().expect("Already built bldc"),
            lowa_highc_config,
        );
        let highb_higha_pwm = Pwm::new_output_ab(
            self.pwm_slice_highb_highc
                .take()
                .expect("Already built bldc"),
            self.highb_pin.take().expect("Already built bldc"),
            self.higha_pin.take().expect("Already built bldc"),
            highb_higha_config,
        );

        let (lowc_ch, lowb_ch) = lowc_lowb_pwm.split();
        let (lowa_ch, highc_ch) = lowa_highc_pwm.split();
        let (highb_ch, higha_ch) = highb_higha_pwm.split();

        let phase_a = drivers::bldc::Phase::new(
            lowa_ch.expect("Phase A low split failed"),
            higha_ch.expect("Phase A high split failed"),
        );
        let phase_b = drivers::bldc::Phase::new(
            lowb_ch.expect("Phase B low split failed"),
            highb_ch.expect("Phase B high split failed"),
        );
        let phase_c = drivers::bldc::Phase::new(
            lowc_ch.expect("Phase C low split failed"),
            highc_ch.expect("Phase C high split failed"),
        );

        self.bldc = Some(drivers::bldc::BLDC::new(phase_a, phase_b, phase_c));
    }

    fn build_rgb(&mut self) {
        let mut led_cfg = pwm::Config::default();
        led_cfg.top = LED_PWM_TOP;
        led_cfg.enable = true;

        let red_green_pwm = Pwm::new_output_ab(
            self.pwm_slice_red_green.take().expect("Already Built RGB"),
            self.red_pin.take().expect("Already Built RGB"),
            self.green_pin.take().expect("Already Built RGB"),
            led_cfg.clone(),
        );
        let blue_pwm = Pwm::new_output_a(
            self.pwm_slice_blue.take().expect("Already Built RGB"),
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
