#![no_std]
#![no_main]

use cortex_m::prelude::_embedded_hal_adc_OneShot;
use defmt::*;
use defmt_rtt as _;
use panic_probe as _;
use rp235x_hal::adc::AdcPin;
use rp235x_hal::clocks::init_clocks_and_plls;
use rp235x_hal::pwm::Slices;
use rp235x_hal::{self as hal, Adc, entry};
use rp235x_hal::{Clock, pac};

mod drivers;
pub mod utils;
mod v2_control_board;

use v2_control_board::FireAntBoard;

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
    let (mut board, gpio29) = FireAntBoard::new(pwm, pins);

    let mut adc = Adc::new(pac.ADC, &mut pac.RESETS);
    let mut adc_pin_0 = AdcPin::new(gpio29.into_floating_input()).unwrap();
    adc.free_running(&adc_pin_0);

    board.rgb.green();
    loop {
        board.bldc.progress();
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
