use crate::drivers::{bldc, rgb_led};
use rp235x_hal::gpio::{FunctionNull, Pin, Pins, PullDown, bank0::Gpio29};
use rp235x_hal::pwm::{
    A, B, Channel, FreeRunning, Pwm0, Pwm1, Pwm5, Pwm6, Pwm7, Slice, SliceId, Slices,
};

pub type RGBRed = Channel<Slice<Pwm0, FreeRunning>, A>;
pub type RGBGreen = Channel<Slice<Pwm0, FreeRunning>, B>;
pub type RGBBlue = Channel<Slice<Pwm1, FreeRunning>, A>;

pub type RGBLed = crate::drivers::rgb_led::RGBLed<RGBRed, RGBGreen, RGBBlue>;

pub type BLDCLowC = Channel<Slice<Pwm5, FreeRunning>, A>;
pub type BLDCLowB = Channel<Slice<Pwm5, FreeRunning>, B>;
pub type BLDCLowA = Channel<Slice<Pwm6, FreeRunning>, A>;

pub type BLDCHighC = Channel<Slice<Pwm6, FreeRunning>, B>;
pub type BLDCHighB = Channel<Slice<Pwm7, FreeRunning>, A>;
pub type BLDCHighA = Channel<Slice<Pwm7, FreeRunning>, B>;

pub type BLDC =
    crate::drivers::bldc::BLDC<BLDCLowA, BLDCLowB, BLDCLowC, BLDCHighA, BLDCHighB, BLDCHighC>;

const LED_TOP: u16 = 65535;
const BLDC_TOP: u16 = 7499;

pub struct FireAntBoard {
    pub rgb: RGBLed,
    pub bldc: BLDC,
}

impl FireAntBoard {
    pub fn new(pwm: Slices, mut pins: Pins) -> (Self, Pin<Gpio29, FunctionNull, PullDown>) {
        fn configure_slice<T: SliceId>(slice: &mut Slice<T, FreeRunning>, top: u16) {
            slice.set_top(top);
            slice.enable();
        }

        let gpio29 = pins.gpio29;

        let mut red_green_slice = pwm.pwm0;
        let mut blue_lphase_slice = pwm.pwm1;

        configure_slice(&mut red_green_slice, LED_TOP);
        configure_slice(&mut blue_lphase_slice, LED_TOP);

        let mut red_channel: RGBRed = red_green_slice.channel_a;
        let mut green_channel: RGBGreen = red_green_slice.channel_b;
        let mut blue_channel: RGBBlue = blue_lphase_slice.channel_a;

        let _ = blue_channel.output_to(pins.gpio18);
        let _ = green_channel.output_to(pins.gpio17);
        let _ = red_channel.output_to(pins.gpio16);

        let rgb = rgb_led::RGBLed::new(red_channel, green_channel, blue_channel);

        let mut lowc_lowb_slice = pwm.pwm5;
        let mut lowa_highc_slice = pwm.pwm6;
        let mut highb_highc_slice = pwm.pwm7;

        configure_slice(&mut lowc_lowb_slice, BLDC_TOP);
        configure_slice(&mut lowa_highc_slice, BLDC_TOP);
        configure_slice(&mut highb_highc_slice, BLDC_TOP);

        let mut lowc_channel = lowc_lowb_slice.channel_a;
        let mut lowb_channel = lowc_lowb_slice.channel_b;
        let mut lowa_channel = lowa_highc_slice.channel_a;
        let mut highc_channel = lowa_highc_slice.channel_b;
        let mut highb_channel = highb_highc_slice.channel_a;
        let mut higha_channel = highb_highc_slice.channel_b;

        let _ = lowc_channel.output_to(pins.gpio10);
        let _ = lowb_channel.output_to(pins.gpio11);
        let _ = lowa_channel.output_to(pins.gpio12);
        let _ = highc_channel.output_to(pins.gpio13);
        let _ = highb_channel.output_to(pins.gpio14);
        let _ = higha_channel.output_to(pins.gpio15);

        let bldc = bldc::BLDC::new(
            lowa_channel,
            lowb_channel,
            lowc_channel,
            higha_channel,
            highb_channel,
            highc_channel,
        );

        (Self { rgb, bldc }, gpio29)
    }
}
