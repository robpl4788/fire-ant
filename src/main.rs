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

use v2_control_board::FireAntBoard;

bind_interrupts!(
    struct Irqs {
        ADC_IRQ_FIFO => InterruptHandler;
    }
);

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

    let mut adc = Adc::new(p.ADC, Irqs, AdcConfig::default());
    let mut adc_pin_0 = Channel::new_pin(p.PIN_29, Pull::None);

    board.rgb.green();
    loop {
        board.bldc.progress();
        let pin_adc_counts = adc.read(&mut adc_pin_0).await.unwrap();
        info!("ADC counts: {}", pin_adc_counts);
        Timer::after_millis(10).await;
    }
}
