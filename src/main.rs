#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::adc::{
    Adc, Async, Channel, Config as AdcConfig, InterruptHandler as ADCInterruptHandler,
};
use embassy_rp::clocks::ClockConfig;
use embassy_rp::config::Config as RpConfig;
use embassy_rp::gpio::{Level, Output, Pull};
use embassy_rp::pac;
use embassy_rp::peripherals::USB;
use embassy_rp::pwm::Pwm;
// use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_rp::{bind_interrupts, pwm};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embassy_time::{Instant, Timer};
// use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
// use embassy_usb::driver::EndpointError;
use panic_probe as _;
use static_cell::StaticCell;

mod drivers;
pub mod utils;

// // Timing constants
// const USB_BUFFER_SIZE: usize = 64;
// const USB_PACKET_COUNT: usize = 4;

// // USB configuration constants
// const USB_VID: u16 = 0x0000;
// const USB_PID: u16 = 0x0000;
// const USB_MANUFACTURER: &str = "Robert";
// const USB_PRODUCT: &str = "FireAntBoard";
// const USB_SERIAL: &str = "12345678";
// const USB_MAX_POWER: u16 = 100;
// const USB_MAX_PACKET_SIZE: u8 = 64;

// PWM frequency configurations
const LED_PWM_TOP: u16 = 65535;
const BLDC_PWM_TOP: u16 = 7499;

static PWM_TOP_IRQ_FIRED: Signal<CriticalSectionRawMutex, ()> = Signal::new();

bind_interrupts!(struct ADCIrqs { ADC_IRQ_FIFO => ADCInterruptHandler; });
// bind_interrupts!(struct USBIrqs { USBCTRL_IRQ => InterruptHandler<USB>; });

// Type aliases for cleaner signatures
// type UsbDriver = Driver<'static, USB>;
// type UsbDevice = embassy_usb::UsbDevice<'static, UsbDriver>;
// type UsbCdcClass = CdcAcmClass<'static, UsbDriver>;

pub type ADCMutex = Mutex<CriticalSectionRawMutex, Adc<'static, Async>>;
pub static ADC_CELL: StaticCell<ADCMutex> = StaticCell::new();

// USB Stuff
// #[embassy_executor::task]
// async fn usb_task(mut usb: UsbDevice) {
//     usb.run().await
// }
//
// #[embassy_executor::task]
// async fn usb_monitor(mut class: UsbCdcClass) {
//     loop {
//         class.wait_connection().await;
//         info!("USB connected");
//         let _ = usb_log_task(&mut class).await;
//         info!("USB disconnected");
//     }
// }
//
// Disconnected state for USB endpoint errors
// #[derive(Debug)]
// struct Disconnected;
//
// impl From<EndpointError> for Disconnected {
//     fn from(err: EndpointError) -> Self {
//         match err {
//             EndpointError::BufferOverflow => defmt::panic!("USB buffer overflow"),
//             EndpointError::Disabled => Disconnected,
//         }
//     }
// }
//
// /// Log data via USB, splitting large buffers into USB packet-sized chunks
// async fn usb_log_task(class: &mut UsbCdcClass) -> Result<(), Disconnected> {
//     loop {
//         Timer::after_millis(20).await;
//         let data = {
//             let mut logger = LOGGER.lock().await;
//             logger.get_data()
//         };
//         // Send data in USB_BUFFER_SIZE chunks
//         for chunk in data.chunks(USB_BUFFER_SIZE) {
//             class.write_packet(chunk).await?;
//         }
//         class.write_packet(&[]).await?; // Flush
//     }
// }
//
// /// Initialize USB device and CDC class with embassy defaults
// fn setup_usb_device(usb_driver: UsbDriver) -> (UsbDevice, UsbCdcClass) {
//     let config = {
//         let mut cfg = embassy_usb::Config::new(USB_VID, USB_PID);
//         cfg.manufacturer = Some(USB_MANUFACTURER);
//         cfg.product = Some(USB_PRODUCT);
//         cfg.serial_number = Some(USB_SERIAL);
//         cfg.max_power = USB_MAX_POWER;
//         cfg.max_packet_size_0 = USB_MAX_PACKET_SIZE;
//         cfg
//     };
//     let mut builder = {
//         static CONFIG_DESC: StaticCell<[u8; 256]> = StaticCell::new();
//         static BOS_DESC: StaticCell<[u8; 256]> = StaticCell::new();
//         static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
//         embassy_usb::Builder::new(
//             usb_driver,
//             config,
//             CONFIG_DESC.init([0; 256]),
//             BOS_DESC.init([0; 256]),
//             &mut [],
//             CONTROL_BUF.init([0; 64]),
//         )
//     };
//
//     let class = {
//         static STATE: StaticCell<State> = StaticCell::new();
//         CdcAcmClass::new(
//             &mut builder,
//             STATE.init(State::new()),
//             USB_BUFFER_SIZE as u16,
//         )
//     };
//     let usb_device = builder.build();
//     (usb_device, class)
// }

async fn get_phase_voltage(adc_pin: &mut Channel<'_>, adc: &mut Adc<'static, Async>) -> f32 {
    let raw_voltage: Option<u16> = adc.read(adc_pin).await.ok();
    // println!("{}", raw_voltage);
    if let Some(voltage) = raw_voltage {
        let voltage = voltage as f32 / 4096.0 * 3.3 * 3.0;
        voltage
    } else {
        defmt::panic!();
    }
}

pub async fn get_current_amps(adc_pin: &mut Channel<'_>, adc: &mut Adc<'static, Async>) -> f32 {
    let current_amps: f32;
    let current_raw: u16;
    current_raw = adc.read(adc_pin).await.unwrap();
    current_amps = (current_raw as f32 - 2048.0) / 2048.0 * 30.0;

    current_amps
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Fire Ant Control Board starting...");

    // Initialize hardware
    let mut config = RpConfig::default();
    config.clocks = ClockConfig::crystal(12_000_000);
    let peripherals = embassy_rp::init(config);

    // Main control loop: ADC sampling
    let mut adc = Adc::new(peripherals.ADC, ADCIrqs, AdcConfig::default());
    // let adc_mutex_ref: &mut ADCMutex = ADC_CELL.init(Mutex::new(adc));

    // Setup board components

    let pwm_rg = peripherals.PWM_SLICE0;
    let red_pin = peripherals.PIN_16;
    let green_pin = peripherals.PIN_17;
    let pwm_b = peripherals.PWM_SLICE1;
    let blue_pin = peripherals.PIN_18;

    let pwm_lowc_lowb = peripherals.PWM_SLICE5;
    let lowc_pin = peripherals.PIN_10;
    let lowb_pin = peripherals.PIN_11;
    let pwm_lowa_highc = peripherals.PWM_SLICE6;
    let lowa_pin = peripherals.PIN_12;
    let highc_pin = peripherals.PIN_13;
    let pwm_highb_higha = peripherals.PWM_SLICE7;
    let highb_pin = peripherals.PIN_14;
    let higha_pin = peripherals.PIN_15;

    let bemf_common_pin = peripherals.PIN_26;
    let bemf_b_pin = peripherals.PIN_27;
    let bemf_a_pin = peripherals.PIN_28;
    let current_sense_pin = peripherals.PIN_29;

    let mut led_cfg = pwm::Config::default();
    led_cfg.top = LED_PWM_TOP;

    let red_green_pwm = Pwm::new_output_ab(pwm_rg, red_pin, green_pin, led_cfg.clone());
    let blue_pwm = Pwm::new_output_a(pwm_b, blue_pin, led_cfg);

    let (red_ch, green_ch) = red_green_pwm.split();
    let (blue_ch, _) = blue_pwm.split();

    let mut rgb = drivers::rgb_led::RGBLed::new(
        red_ch.expect("Red channel split failed"),
        green_ch.expect("Green channel split failed"),
        blue_ch.expect("Blue channel split failed"),
    );

    let mut bldc_cfg = pwm::Config::default();
    bldc_cfg.top = BLDC_PWM_TOP;
    bldc_cfg.enable = false;

    let mut lowc_lowb_config = bldc_cfg.clone();
    lowc_lowb_config.invert_a = true;
    lowc_lowb_config.invert_b = true;

    let mut lowa_highc_config = bldc_cfg.clone();
    lowa_highc_config.invert_a = true;

    let highb_higha_config = bldc_cfg.clone();

    let lowc_lowb_pwm = Pwm::new_output_ab(pwm_lowc_lowb, lowc_pin, lowb_pin, lowc_lowb_config);
    let lowa_highc_pwm = Pwm::new_output_ab(pwm_lowa_highc, lowa_pin, highc_pin, lowa_highc_config);
    let highb_higha_pwm =
        Pwm::new_output_ab(pwm_highb_higha, highb_pin, higha_pin, highb_higha_config);

    let mut ticker_pwm = Pwm::new_output_ab(
        peripherals.PWM_SLICE4,
        peripherals.PIN_24,
        peripherals.PIN_25,
        bldc_cfg,
    );

    pac::PWM.en().write(|w| {
        w.set_ch4(true);
        w.set_ch5(true);
        w.set_ch6(true);
        w.set_ch7(true);
    });

    let (lowc_ch, lowb_ch) = lowc_lowb_pwm.split();
    let (lowa_ch, highc_ch) = lowa_highc_pwm.split();
    let (highb_ch, higha_ch) = highb_higha_pwm.split();

    let phase_a = drivers::bldc::Phase::new(
        lowa_ch.expect("Phase A low split failed"),
        higha_ch.expect("Phase A high split failed"),
    );
    let phase_b = drivers::bldc::Phase::new(
        lowb_ch.expect("Phase B low split failed"),
        highb_ch.expect("Phase B high split failed"),
    );
    let phase_c = drivers::bldc::Phase::new(
        lowc_ch.expect("Phase C low split failed"),
        highc_ch.expect("Phase C high split failed"),
    );

    let mut bemf_common_pin: Channel<'_> = Channel::new_pin(bemf_common_pin, Pull::None);
    let mut bemf_b_pin = Channel::new_pin(bemf_b_pin, Pull::None);
    let mut bemf_a_pin = Channel::new_pin(bemf_a_pin, Pull::None);
    let mut current_sense_pin = Channel::new_pin(current_sense_pin, Pull::None);

    let mut bldc = drivers::bldc::BLDC::new(phase_a, phase_b, phase_c);

    // Setup USB
    // let usb_driver = Driver::new(peripherals.USB, USBIrqs);
    // let (usb_device, usb_class) = setup_usb_device(usb_driver);

    // spawner.spawn(usb_task(usb_device).expect("Failed to create USB task"));
    // spawner.spawn(usb_monitor(usb_class).expect("Failed to create USB monitor task"));

    rgb.red();

    bldc.disable();
    // spawner.spawn(run_motor(board).expect("Failed to create USB monitor task"));

    let mut target_rps: i16 = 1;
    let mut direction: i16 = 1;

    let _l_phase = Output::new(peripherals.PIN_19, Level::Low);
    let mut l_enable = Output::new(peripherals.PIN_20, Level::High);

    let mut last_loop: u64 = 0;
    loop {
        for _ in 0..800 {
            ticker_pwm.wait_for_wrap();
            let now = Instant::now().as_micros();
            // println!("dt: {}", &now - last_loop);
            last_loop = now;
            if now < 2_000_000 {
                bldc.open_loop(now).await;
                l_enable.set_low();
            } else if now < 3_000_000 {
                Timer::after_micros(2).await;
                let v_a = get_phase_voltage(&mut bemf_a_pin, &mut adc).await;
                let v_com = get_phase_voltage(&mut bemf_common_pin, &mut adc).await;
                // let d_t = Instant::now().as_micros() - bldc.get_last_commutation();
                // println!("v_a: {}, v_com: {}", v_a, v_com);
                l_enable.set_high();
                bldc.closed_loop(v_a, v_com).await;
            } else {
                bldc.disable();
                defmt::panic!();
            }
        }

        if target_rps >= 50 {
            direction = 0;
        } else if target_rps <= 0 {
            direction = 0;
        } else {
            target_rps += direction;
            bldc.set_target_rps(target_rps);
        }

        // println!("target_rps: {}", &target_rps);
    }
}

/// Program metadata for `picotool info`
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 2] = [
    embassy_rp::binary_info::rp_program_name!(c"Fire Ant Control Board"),
    embassy_rp::binary_info::rp_program_description!(c"Version 0.1.0"),
];
