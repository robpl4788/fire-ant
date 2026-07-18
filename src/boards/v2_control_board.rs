use embassy_rp::clocks::ClockConfig;
use embassy_rp::config::Config as RpConfig;
use embassy_rp::gpio::{Input, Output};
use embassy_rp::peripherals::{self, SPI0};
use embassy_rp::pwm::{self, Pwm};
use embassy_rp::{Peri, spi};

use crate::drivers;
use crate::drivers::bldc::BLDC;
use crate::drivers::radio::radio::Radio;
use crate::drivers::rgb_led::{self, RGBLed};

pub type RGBLedType =
    RGBLed<pwm::PwmOutput<'static>, pwm::PwmOutput<'static>, pwm::PwmOutput<'static>>;
pub type BLDCType = BLDC<
    pwm::PwmOutput<'static>,
    pwm::PwmOutput<'static>,
    pwm::PwmOutput<'static>,
    pwm::PwmOutput<'static>,
    pwm::PwmOutput<'static>,
    pwm::PwmOutput<'static>,
>;
pub type RadioType = Radio<SPI0>;

const LED_PWM_TOP: u16 = 65535;
const BLDC_PWM_TOP: u16 = 7499;

pub struct V2ControlBoard {
    rgb: Option<RGBLedType>,
    bldc: Option<BLDCType>,
    radio: Option<Radio<SPI0>>,

    pwm_slice_red_green: Option<Peri<'static, peripherals::PWM_SLICE0>>,
    pwm_slice_blue: Option<Peri<'static, peripherals::PWM_SLICE1>>,

    red_pin: Option<Peri<'static, peripherals::PIN_16>>,
    green_pin: Option<Peri<'static, peripherals::PIN_17>>,
    blue_pin: Option<Peri<'static, peripherals::PIN_18>>,

    pwm_slice_lowc_lowb: Option<Peri<'static, peripherals::PWM_SLICE5>>,
    pwm_slice_lowa_highc: Option<Peri<'static, peripherals::PWM_SLICE6>>,
    pwm_slice_highb_highc: Option<Peri<'static, peripherals::PWM_SLICE7>>,

    lowa_pin: Option<Peri<'static, peripherals::PIN_12>>,
    lowb_pin: Option<Peri<'static, peripherals::PIN_11>>,
    lowc_pin: Option<Peri<'static, peripherals::PIN_10>>,
    higha_pin: Option<Peri<'static, peripherals::PIN_15>>,
    highb_pin: Option<Peri<'static, peripherals::PIN_14>>,
    highc_pin: Option<Peri<'static, peripherals::PIN_13>>,

    miso_pin: Option<Peri<'static, peripherals::PIN_4>>,
    mosi_pin: Option<Peri<'static, peripherals::PIN_3>>,
    clk_pin: Option<Peri<'static, peripherals::PIN_2>>,
    radio_cs_n_pin: Option<Peri<'static, peripherals::PIN_1>>,
    busy_pin: Option<Peri<'static, peripherals::PIN_8>>,
    dio1_pin: Option<Peri<'static, peripherals::PIN_7>>,
    dio2_pin: Option<Peri<'static, peripherals::PIN_6>>,
    dio3_pin: Option<Peri<'static, peripherals::PIN_5>>,
    radio_spi: Option<Peri<'static, peripherals::SPI0>>,
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
            radio: None,

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

            miso_pin: Some(peripherals.PIN_4),
            mosi_pin: Some(peripherals.PIN_3),
            clk_pin: Some(peripherals.PIN_2),
            radio_cs_n_pin: Some(peripherals.PIN_1),
            busy_pin: Some(peripherals.PIN_8),
            dio1_pin: Some(peripherals.PIN_7),
            dio2_pin: Some(peripherals.PIN_6),
            dio3_pin: Some(peripherals.PIN_5),
            radio_spi: Some(peripherals.SPI0),
        };

        result.build_rgb();
        result.build_bldc();
        result.build_radio();

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

    fn build_radio(&mut self) {
        // create SPI
        let mut config = spi::Config::default();
        config.frequency = 2_000_000;
        config.phase = spi::Phase::CaptureOnFirstTransition;
        config.polarity = spi::Polarity::IdleLow;
        let spi = spi::Spi::new_blocking(
            self.radio_spi.take().expect("Already built radio"),
            self.clk_pin.take().expect("Already built radio"),
            self.mosi_pin.take().expect("Already built radio"),
            self.miso_pin.take().expect("Already built radio"),
            config,
        );

        let radio_cs_n_output = Output::new(
            self.radio_cs_n_pin.take().expect("Already built radio"),
            embassy_rp::gpio::Level::High,
        );

        let busy = Input::new(
            self.busy_pin.take().expect("Already built radio"),
            embassy_rp::gpio::Pull::None,
        );

        let dio1 = Input::new(
            self.dio1_pin.take().expect("Already built radio"),
            embassy_rp::gpio::Pull::None,
        );

        let dio2 = Input::new(
            self.dio2_pin.take().expect("Already built radio"),
            embassy_rp::gpio::Pull::None,
        );

        let dio3 = Input::new(
            self.dio3_pin.take().expect("Already built radio"),
            embassy_rp::gpio::Pull::None,
        );

        self.radio = Some(Radio::new(spi, radio_cs_n_output, busy, dio1, dio2, dio3));
    }

    pub fn take_rgb(&mut self) -> RGBLedType {
        let result = self.rgb.take();
        result.expect("Already took RGB Led")
    }

    pub fn take_bldc(&mut self) -> BLDCType {
        let result = self.bldc.take();
        result.expect("Already took BLDC")
    }

    pub fn take_radio(&mut self) -> RadioType {
        let result = self.radio.take();
        result.expect("Already took radio")
    }
}
