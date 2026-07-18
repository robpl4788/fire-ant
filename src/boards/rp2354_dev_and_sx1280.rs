use embassy_rp::clocks::ClockConfig;
use embassy_rp::config::Config as RpConfig;
use embassy_rp::gpio::{Input, Output};
use embassy_rp::peripherals::{self, SPI0};
use embassy_rp::pwm::{self, Pwm};
use embassy_rp::{Peri, spi};
use embassy_time::Timer;

use crate::drivers::radio::radio::Radio;
use crate::drivers::rgb_led::{self, RGBLed};

pub type RGBLedType =
    RGBLed<pwm::PwmOutput<'static>, pwm::PwmOutput<'static>, pwm::PwmOutput<'static>>;

pub type RadioType = Radio<SPI0>;

const LED_PWM_TOP: u16 = 65535;

pub struct Rp2354DevAndSx1280 {
    rgb: Option<RGBLedType>,
    radio: Option<Radio<SPI0>>,

    pwm_slice_red_green: Option<Peri<'static, peripherals::PWM_SLICE2>>,
    pwm_slice_blue: Option<Peri<'static, peripherals::PWM_SLICE3>>,

    red_pin: Option<Peri<'static, peripherals::PIN_4>>,
    green_pin: Option<Peri<'static, peripherals::PIN_5>>,
    blue_pin: Option<Peri<'static, peripherals::PIN_6>>,

    miso_pin: Option<Peri<'static, peripherals::PIN_0>>,
    mosi_pin: Option<Peri<'static, peripherals::PIN_3>>,
    clk_pin: Option<Peri<'static, peripherals::PIN_2>>,
    radio_cs_n_pin: Option<Peri<'static, peripherals::PIN_1>>,
    busy_pin: Option<Peri<'static, peripherals::PIN_15>>,
    dio1_pin: Option<Peri<'static, peripherals::PIN_14>>,
    dio2_pin: Option<Peri<'static, peripherals::PIN_13>>,
    dio3_pin: Option<Peri<'static, peripherals::PIN_12>>,
    radio_spi: Option<Peri<'static, peripherals::SPI0>>,
}

impl Rp2354DevAndSx1280 {
    pub async fn new() -> Self {
        // Initialize hardware
        let mut config = RpConfig::default();
        config.clocks = ClockConfig::crystal(12_000_000);
        let peripherals = embassy_rp::init(config);

        let mut result = Rp2354DevAndSx1280 {
            rgb: None,
            radio: None,

            red_pin: Some(peripherals.PIN_4),
            green_pin: Some(peripherals.PIN_5),
            blue_pin: Some(peripherals.PIN_6),

            pwm_slice_red_green: Some(peripherals.PWM_SLICE2),
            pwm_slice_blue: Some(peripherals.PWM_SLICE3),

            miso_pin: Some(peripherals.PIN_0),
            mosi_pin: Some(peripherals.PIN_3),
            clk_pin: Some(peripherals.PIN_2),
            radio_cs_n_pin: Some(peripherals.PIN_1),
            busy_pin: Some(peripherals.PIN_15),
            dio1_pin: Some(peripherals.PIN_14),
            dio2_pin: Some(peripherals.PIN_13),
            dio3_pin: Some(peripherals.PIN_12),
            radio_spi: Some(peripherals.SPI0),
        };

        let reset_radio_pin = peripherals.PIN_11;
        let mut reset_radio = Output::new(reset_radio_pin, embassy_rp::gpio::Level::Low);

        Timer::after_millis(10).await;

        reset_radio.set_high();

        Timer::after_millis(10).await;

        result.build_rgb();
        result.build_radio();

        result.enable_pwm();

        result
    }

    fn enable_pwm(&mut self) {
        embassy_rp::pac::PWM.en().write(|w| {
            w.set_ch0(true);
            w.set_ch1(true);
            w.set_ch2(true);
            w.set_ch3(true);
            w.set_ch4(true);
            w.set_ch5(true);
            w.set_ch6(true);
            w.set_ch7(true);
        });
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

    pub fn take_radio(&mut self) -> RadioType {
        let result = self.radio.take();
        result.expect("Already took radio")
    }
}
