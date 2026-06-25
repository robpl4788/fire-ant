#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::adc::{Adc, Channel, Config as AdcConfig, InterruptHandler as ADCInterruptHandler};
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::ClockConfig;
use embassy_rp::config::Config as RpConfig;
use embassy_rp::gpio::Pull;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, Instance, InterruptHandler};
use embassy_time::Timer;
use embassy_usb::UsbDevice;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;
use panic_probe as _;
use static_cell::StaticCell;

mod drivers;
pub mod utils;
mod v2_control_board;

use crate::drivers::logger::LOGGER;
use v2_control_board::FireAntBoard;

bind_interrupts!(
    struct ADCIrqs {
        ADC_IRQ_FIFO => ADCInterruptHandler;
    }
);

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::task]
async fn run_motor(mut board: FireAntBoard) {
    loop {
        board.bldc.progress();
        Timer::after_millis(10).await;
    }
}

type MyUsbDriver = Driver<'static, USB>;
type MyUsbDevice = UsbDevice<'static, MyUsbDriver>;

#[embassy_executor::task]
async fn usb_task(mut usb: MyUsbDevice) -> ! {
    usb.run().await
}

#[embassy_executor::task]
async fn usb_monitor(mut class: CdcAcmClass<'static, Driver<'static, USB>>) {
    // Do stuff with the class!
    loop {
        class.wait_connection().await;
        info!("Connected");
        let _ = log(&mut class).await;
        info!("Disconnected");
    }
}

struct Disconnected {}

impl From<EndpointError> for Disconnected {
    fn from(val: EndpointError) -> Self {
        match val {
            EndpointError::BufferOverflow => defmt::panic!("Buffer overflow"),
            EndpointError::Disabled => Disconnected {},
        }
    }
}

async fn log<'d, T: Instance + 'd>(
    class: &mut CdcAcmClass<'d, Driver<'d, T>>,
) -> Result<(), Disconnected> {
    loop {
        Timer::after_millis(20).await;
        let data: [u8; 256];
        {
            let mut logger = LOGGER.lock().await;
            data = logger.get_data();
        }

        // Split data into parts to fit in usb buffer
        let a = &data[0..64];
        let b = &data[64..128];
        let c = &data[128..192];
        let d = &data[192..256];

        class.write_packet(&a).await?;
        class.write_packet(&b).await?;
        class.write_packet(&c).await?;
        class.write_packet(&d).await?;
        class.write_packet(&[]).await?; // Flush the terminal
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

    // Create the driver, from the HAL.
    let usb_driver = Driver::new(p.USB, Irqs);

    // Create embassy-usb Config
    let config = {
        let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
        config.manufacturer = Some("Embassy");
        config.product = Some("USB-serial example");
        config.serial_number = Some("12345678");
        config.max_power = 100;
        config.max_packet_size_0 = 64;
        config
    };

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut builder = {
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

        let builder = embassy_usb::Builder::new(
            usb_driver,
            config,
            CONFIG_DESCRIPTOR.init([0; 256]),
            BOS_DESCRIPTOR.init([0; 256]),
            &mut [], // no msos descriptors
            CONTROL_BUF.init([0; 64]),
        );
        builder
    };

    // Create classes on the builder.
    let class: CdcAcmClass<'_, Driver<'_, USB>> = {
        static STATE: StaticCell<State> = StaticCell::new();
        let state = STATE.init(State::new());
        CdcAcmClass::new(&mut builder, state, 64)
    };

    // Build the builder.
    let usb = builder.build();

    // Run the USB device.
    _spawner.spawn(unwrap!(usb_task(usb)));
    _spawner.spawn(unwrap!(usb_monitor(class)));

    board.rgb.green();

    board.bldc.disable();
    // _spawner.spawn(run_motor(board).unwrap());

    let mut adc = Adc::new(p.ADC, ADCIrqs, AdcConfig::default());
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
