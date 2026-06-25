#![no_std]
#![no_main]

use cortex_m::prelude::_embedded_hal_adc_OneShot;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::pwm;
use panic_probe as _;
use rp235x_hal::adc::AdcPin;
use rp235x_hal::clocks::init_clocks_and_plls;
use rp235x_hal::gpio::Pins;
use rp235x_hal::pwm::Slices;
use rp235x_hal::{self as hal, Adc, Timer, entry};
use rp235x_hal::{Clock, pac};

mod drivers;
pub mod utils;
mod v2_control_board;

use drivers::{bldc, rgb_led};

/// Tell the Boot ROM about our application
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let sio = hal::Sio::new(pac.SIO);

    // External high-speed crystal on the fire ant control board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let pwm = Slices::new(pac.PWM, &mut pac.RESETS);

    // Create the RGB LED driver
    let mut red_green_slice = pwm.pwm0;
    let mut blue_lphase_slice = pwm.pwm1;

    let led_top = 65535;

    red_green_slice.set_top(led_top);
    red_green_slice.enable();

    blue_lphase_slice.set_top(led_top);
    blue_lphase_slice.enable();

    let mut red_channel: v2_control_board::RGBRed = red_green_slice.channel_a;
    let mut green_channel: v2_control_board::RGBGreen = red_green_slice.channel_b;
    let mut blue_channel: v2_control_board::RGBBlue = blue_lphase_slice.channel_a;

    let _blue_pin = blue_channel.output_to(pins.gpio18);
    let _green_pin = green_channel.output_to(pins.gpio17);
    let _red_pin = red_channel.output_to(pins.gpio16);

    let mut rgb: v2_control_board::RGBLed =
        rgb_led::RGBLed::new(red_channel, green_channel, blue_channel);

    let mut lowc_lowb_slice = pwm.pwm5;
    let mut lowa_highc_slice = pwm.pwm6;
    let mut highb_highc_slice = pwm.pwm7;

    let bldc_top = 65535;

    lowc_lowb_slice.set_top(bldc_top);
    lowc_lowb_slice.enable();

    lowa_highc_slice.set_top(bldc_top);
    lowa_highc_slice.enable();

    highb_highc_slice.set_top(bldc_top);
    highb_highc_slice.enable();

    let mut lowc_channel = lowc_lowb_slice.channel_a;
    let mut lowb_channel = lowc_lowb_slice.channel_b;
    let mut lowa_channel = lowa_highc_slice.channel_a;
    let mut highc_channel = lowa_highc_slice.channel_b;
    let mut highb_channel = highb_highc_slice.channel_a;
    let mut higha_channel = highb_highc_slice.channel_b;

    let _lowc_pin = lowc_channel.output_to(pins.gpio10);
    let _lowb_pin = lowb_channel.output_to(pins.gpio11);
    let _lowa_pin = lowa_channel.output_to(pins.gpio12);
    let _highc_pin = highc_channel.output_to(pins.gpio13);
    let _highb_pin = highb_channel.output_to(pins.gpio14);
    let _higha_pin = higha_channel.output_to(pins.gpio15);

    let mut bldc: v2_control_board::BLDC = bldc::BLDC::new(
        lowa_channel,
        lowb_channel,
        lowc_channel,
        higha_channel,
        highb_channel,
        highc_channel,
    );

    let mut timer = Timer::new_timer0(pac.TIMER0, &mut pac.RESETS, &clocks);

    let mut adc = Adc::new(pac.ADC, &mut pac.RESETS);
    let mut adc_pin_0 = AdcPin::new(pins.gpio29.into_floating_input()).unwrap();
    adc.free_running(&adc_pin_0);

    let mut last_change = timer.get_counter().ticks();
    rgb.green();
    loop {
        // info!("full on!");
        bldc.progress();
        // bldc.disable();
        let pin_adc_counts: u16 = adc.read(&mut adc_pin_0).unwrap();
        info!("ADC counts: {}", pin_adc_counts);
        delay.delay_ms(10);
    }
}

/// Program metadata for `picotool info`
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [rp235x_hal::binary_info::EntryAddr; 5] = [
    rp235x_hal::binary_info::rp_cargo_bin_name!(),
    rp235x_hal::binary_info::rp_cargo_version!(),
    rp235x_hal::binary_info::rp_program_description!(c"Fire Ant Control Board"),
    rp235x_hal::binary_info::rp_cargo_homepage_url!(),
    rp235x_hal::binary_info::rp_program_build_attribute!(),
];

// End of file
