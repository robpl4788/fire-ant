#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::adc::{Adc, Channel, Config as AdcConfig, InterruptHandler};
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::ClockConfig;
use embassy_rp::config::Config as RpConfig;
use embassy_rp::gpio::Pull;
use embassy_time::Timer;
use panic_probe as _;

mod drivers;
pub mod utils;
mod v2_control_board;

use crate::drivers::logger::LOGGER;
use v2_control_board::FireAntBoard;

bind_interrupts!(
    struct Irqs {
        ADC_IRQ_FIFO => InterruptHandler;
    }
);

#[embassy_executor::task]
async fn output_logs() {
    loop {
        {
            let mut logger = LOGGER.lock().await;
            logger.get_data();
        }
        Timer::after_millis(1000).await;
    }
}

#[embassy_executor::task]
async fn run_motor(mut board: FireAntBoard) {
    loop {
        board.bldc.progress();
        Timer::after_millis(10).await;
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Program start");

    let mut config = RpConfig::default();
    config.clocks = ClockConfig::crystal(12_000_000);

    let p = embassy_rp::init(config);

    let mut board = FireAntBoard::new(
        p.PWM_SLICE0,
        p.PIN_16,
        p.PIN_17,
        p.PWM_SLICE1,
        p.PIN_18,
        p.PWM_SLICE5,
        p.PIN_10,
        p.PIN_11,
        p.PWM_SLICE6,
        p.PIN_12,
        p.PIN_13,
        p.PWM_SLICE7,
        p.PIN_14,
        p.PIN_15,
    );
    board.rgb.green();

    _spawner.spawn(output_logs().unwrap());
    board.bldc.disable();
    // _spawner.spawn(run_motor(board).unwrap());

    let mut adc = Adc::new(p.ADC, Irqs, AdcConfig::default());
    let mut adc_pin_0 = Channel::new_pin(p.PIN_29, Pull::None);

    loop {
        Timer::after_millis(1).await;
        // board.bldc.disable();
        // board.bldc.progress();
        let pin_adc_counts = adc.read(&mut adc_pin_0).await.unwrap();

        let current = (pin_adc_counts as f32 - 2048.0) / 2048.0 * 30.0;
        let mut logger = LOGGER.lock().await;
        logger.log_value("current", current);
        // let info!("ADC counts: {}", pin_adc_counts);
    }
}

/// Program metadata for `picotool info`
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 2] = [
    embassy_rp::binary_info::rp_program_name!(c"Fire Ant Control Board"),
    embassy_rp::binary_info::rp_program_description!(c"Version 0.1.0"),
];
