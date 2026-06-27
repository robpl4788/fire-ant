#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::adc::{
    Adc, Async, Channel, Config as AdcConfig, InterruptHandler as ADCInterruptHandler,
};
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::ClockConfig;
use embassy_rp::config::Config as RpConfig;
use embassy_rp::gpio::{Level, Output, Pull};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Instant, Timer};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;
use panic_probe as _;
use static_cell::StaticCell;

mod drivers;
pub mod utils;
mod v2_control_board;

use crate::drivers::logger::LOGGER;
use crate::v2_control_board::FireAntBoard;
use v2_control_board::FireAntBoardBuilder;

// Timing constants
const ADC_INTERVAL_MS: u64 = 1;
const LOG_INTERVAL_MS: u64 = 2;
const MOTOR_INTERVAL_MS: u64 = 10;
const USB_BUFFER_SIZE: usize = 64;
const USB_PACKET_COUNT: usize = 4;

// USB configuration constants
const USB_VID: u16 = 0x0000;
const USB_PID: u16 = 0x0000;
const USB_MANUFACTURER: &str = "Robert";
const USB_PRODUCT: &str = "FireAntBoard";
const USB_SERIAL: &str = "12345678";
const USB_MAX_POWER: u16 = 100;
const USB_MAX_PACKET_SIZE: u8 = 64;

bind_interrupts!(struct ADCIrqs { ADC_IRQ_FIFO => ADCInterruptHandler; });
bind_interrupts!(struct USBIrqs { USBCTRL_IRQ => InterruptHandler<USB>; });

// Type aliases for cleaner signatures
type UsbDriver = Driver<'static, USB>;
type UsbDevice = embassy_usb::UsbDevice<'static, UsbDriver>;
type UsbCdcClass = CdcAcmClass<'static, UsbDriver>;

pub type ADCMutex = Mutex<CriticalSectionRawMutex, Adc<'static, Async>>;
pub static ADC_CELL: StaticCell<ADCMutex> = StaticCell::new();

#[embassy_executor::task]
async fn usb_task(mut usb: UsbDevice) {
    usb.run().await
}

#[embassy_executor::task]
async fn run_motor(mut board: FireAntBoard) {
    loop {
        board.bldc.progress(0.1);
        Timer::after_millis(MOTOR_INTERVAL_MS).await;
    }
}

#[embassy_executor::task]
async fn usb_monitor(mut class: UsbCdcClass) {
    loop {
        class.wait_connection().await;
        info!("USB connected");
        let _ = usb_log_task(&mut class).await;
        info!("USB disconnected");
    }
}

/// Disconnected state for USB endpoint errors
#[derive(Debug)]
struct Disconnected;

impl From<EndpointError> for Disconnected {
    fn from(err: EndpointError) -> Self {
        match err {
            EndpointError::BufferOverflow => defmt::panic!("USB buffer overflow"),
            EndpointError::Disabled => Disconnected,
        }
    }
}

/// Log data via USB, splitting large buffers into USB packet-sized chunks
async fn usb_log_task(class: &mut UsbCdcClass) -> Result<(), Disconnected> {
    loop {
        Timer::after_millis(LOG_INTERVAL_MS).await;

        let data = {
            let mut logger = LOGGER.lock().await;
            logger.get_data()
        };

        // Send data in USB_BUFFER_SIZE chunks
        for chunk in data.chunks(USB_BUFFER_SIZE) {
            class.write_packet(chunk).await?;
        }

        class.write_packet(&[]).await?; // Flush
    }
}

/// Initialize USB device and CDC class with embassy defaults
fn setup_usb_device(usb_driver: UsbDriver) -> (UsbDevice, UsbCdcClass) {
    let config = {
        let mut cfg = embassy_usb::Config::new(USB_VID, USB_PID);
        cfg.manufacturer = Some(USB_MANUFACTURER);
        cfg.product = Some(USB_PRODUCT);
        cfg.serial_number = Some(USB_SERIAL);
        cfg.max_power = USB_MAX_POWER;
        cfg.max_packet_size_0 = USB_MAX_PACKET_SIZE;
        cfg
    };

    let mut builder = {
        static CONFIG_DESC: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESC: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

        embassy_usb::Builder::new(
            usb_driver,
            config,
            CONFIG_DESC.init([0; 256]),
            BOS_DESC.init([0; 256]),
            &mut [],
            CONTROL_BUF.init([0; 64]),
        )
    };

    let class = {
        static STATE: StaticCell<State> = StaticCell::new();
        CdcAcmClass::new(
            &mut builder,
            STATE.init(State::new()),
            USB_BUFFER_SIZE as u16,
        )
    };

    let usb_device = builder.build();
    (usb_device, class)
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Fire Ant Control Board starting...");

    // Initialize hardware
    let mut config = RpConfig::default();
    config.clocks = ClockConfig::crystal(12_000_000);
    let peripherals = embassy_rp::init(config);

    // Main control loop: ADC sampling
    let adc = Adc::new(peripherals.ADC, ADCIrqs, AdcConfig::default());
    let adc_mutex_ref: &mut ADCMutex = ADC_CELL.init(Mutex::new(adc));

    // Setup board components
    let mut board = FireAntBoardBuilder::new()
        .with_rgb_pwm(
            peripherals.PWM_SLICE0,
            peripherals.PIN_16,
            peripherals.PIN_17,
            peripherals.PWM_SLICE1,
            peripherals.PIN_18,
        )
        .with_bldc_phases(
            peripherals.PWM_SLICE5,
            peripherals.PIN_10,
            peripherals.PIN_11,
            peripherals.PWM_SLICE6,
            peripherals.PIN_12,
            peripherals.PIN_13,
            peripherals.PWM_SLICE7,
            peripherals.PIN_14,
            peripherals.PIN_15,
        )
        .with_adc(
            adc_mutex_ref,
            peripherals.PIN_26,
            peripherals.PIN_27,
            peripherals.PIN_28,
            peripherals.PIN_29,
        )
        .build();

    // Setup USB
    let usb_driver = Driver::new(peripherals.USB, USBIrqs);
    let (usb_device, usb_class) = setup_usb_device(usb_driver);

    // spawner.spawn(usb_task(usb_device).expect("Failed to create USB task"));
    // spawner.spawn(usb_monitor(usb_class).expect("Failed to create USB monitor task"));

    board.rgb.green();

    board.bldc.disable();
    // spawner.spawn(run_motor(board).expect("Failed to create USB monitor task"));

    let mut target_rps: i16 = 1;
    let mut direction: i16 = 1;

    let _l_phase = Output::new(peripherals.PIN_19, Level::Low);
    let mut l_enable = Output::new(peripherals.PIN_20, Level::High);

    loop {
        for _ in 0..400 {
            Timer::after_micros(50).await;
            let now = Instant::now().as_micros();
            if now < 5_000_000 {
                if board.bldc.update(now).await {
                    l_enable.set_high();
                } else {
                    l_enable.set_low();
                }
            } else {
                board.bldc.disable();
                defmt::panic!();
            }
        }

        if target_rps >= 50 {
            direction = 0;
        } else if target_rps <= 0 {
            direction = 0;
        }

        target_rps += direction;

        board.bldc.set_target_rps(target_rps);
        println!("target_rps: {}", &target_rps);
    }
}

/// Program metadata for `picotool info`
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 2] = [
    embassy_rp::binary_info::rp_program_name!(c"Fire Ant Control Board"),
    embassy_rp::binary_info::rp_program_description!(c"Version 0.1.0"),
];
