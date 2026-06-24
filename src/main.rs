#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::OutputPin;
use panic_probe as _;
use rp235x_hal::clocks::init_clocks_and_plls;
use rp235x_hal::pwm::Slices;
use rp235x_hal::{self as hal, entry};
use rp235x_hal::{Clock, pac};

mod drivers;

use drivers::rgb_led;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
// use some_bsp;

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

    let mut pwm = Slices::new(pac.PWM, &mut pac.RESETS);
    let mut red_green_slice = pwm.pwm0;
    let mut blue_lphase_slice = pwm.pwm1;

    let led_top = 65535;

    red_green_slice.set_top(led_top);
    red_green_slice.enable();

    blue_lphase_slice.set_top(led_top);
    blue_lphase_slice.enable();

    let mut red_channel = red_green_slice.channel_a;
    let mut green_channel = red_green_slice.channel_b;
    let mut blue_channel = blue_lphase_slice.channel_a;

    let mut blue_pin = blue_channel.output_to(pins.gpio18);
    let mut green_pin = green_channel.output_to(pins.gpio17);
    let mut red_pin = red_channel.output_to(pins.gpio16);

    let mut rgb = rgb_led::RGBLed::new(red_channel, green_channel, blue_channel, led_top, true);

    let mut x = 0.;
    let mut direction = 0.05;

    loop {
        // info!("full on!");
        rgb.set_rgb(0., x, 0.);
        x += direction;
        if x > 1. {
            x = 1.;
            direction = -direction;
        }

        if x < 0. {
            x = 0.;
            direction = -direction;
        }
        delay.delay_ms(50);
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
