#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};

use panic_probe as _;

use crate::drivers::radio::{self, radio::Radio};

use crate::boards::{rp2354_dev_and_sx1280, v2_control_board};

mod boards;
mod drivers;
pub mod utils;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Fire Ant Control Board starting...");
    // let mut board = v2_control_board::V2ControlBoard::new();

    let mut board = rp2354_dev_and_sx1280::Rp2354DevAndSx1280::new().await;

    let mut rgb = board.take_rgb();
    let radio = board.take_radio();

    rgb.blue();
    Timer::after_millis(2000).await;
    rx(radio, rgb).await;
}

async fn rx(
    mut radio: rp2354_dev_and_sx1280::RadioType,
    mut rgb: rp2354_dev_and_sx1280::RGBLedType,
) {
    loop {
        let data = radio.recieve().await;
        println!("data: {}", &data);

        rgb.green();
        Timer::after_millis(100).await;
        rgb.blue();
    }
}

async fn tx(mut radio: v2_control_board::RadioType, mut rgb: v2_control_board::RGBLedType) {
    let mut x = 0;

    loop {
        let end = Instant::now().saturating_add(Duration::from_millis(500));

        rgb.green();
        radio.transmit(x);

        println!("sent: {}", Instant::now().as_millis());

        while radio.is_tx_done() == false {}
        println!("done: {}", Instant::now().as_millis());
        println!("");

        rgb.blue();

        x = x.wrapping_add(1);
        Timer::at(end).await;
    }
}

/// Program metadata for `picotool info`
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 2] = [
    embassy_rp::binary_info::rp_program_name!(c"Fire Ant Control Board"),
    embassy_rp::binary_info::rp_program_description!(c"Version 0.1.0"),
];
