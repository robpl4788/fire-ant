#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};

use panic_probe as _;

mod v2_control_board;

mod drivers;
pub mod utils;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Fire Ant Control Board starting...");
    let mut board = v2_control_board::V2ControlBoard::new();

    let mut rgb = board.take_rgb();
    let mut radio = board.take_radio();

    rgb.blue();
    Timer::after_millis(2000).await;

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
