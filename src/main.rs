#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_time::Timer;

use panic_probe as _;

use crate::drivers::radio;

mod v2_control_board;

mod drivers;
pub mod utils;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Fire Ant Control Board starting...");
    let mut board = v2_control_board::V2ControlBoard::new();

    let mut rgb = board.take_rgb();
    let mut radio = board.take_radio();

    rgb.blue();

    loop {
        // Timer::after_millis(100).await;
        // if radio.is_busy() == false {
        //     rgb.green();
        // }

        Timer::after_millis(500).await;
        rgb.green();
        radio.set_buffer();
        Timer::after_millis(500).await;
        rgb.blue();
        radio.get_buffer();
    }
}

/// Program metadata for `picotool info`
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 2] = [
    embassy_rp::binary_info::rp_program_name!(c"Fire Ant Control Board"),
    embassy_rp::binary_info::rp_program_description!(c"Version 0.1.0"),
];
