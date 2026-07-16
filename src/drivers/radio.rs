use defmt::println;
use embassy_rp::{
    gpio::{Input, Output},
    peripherals::SPI0,
    spi::{self, Blocking, Spi},
};
use embassy_time::Timer;
use embedded_hal::spi::SpiBus;

pub struct Radio<SPIInstance>
where
    SPIInstance: spi::Instance + 'static,
{
    spi: Spi<'static, SPIInstance, Blocking>,
    n_chip_select: Output<'static>,
    busy: Input<'static>,
    x: u8,
}

impl<SPIInstance> Radio<SPIInstance>
where
    SPIInstance: spi::Instance + 'static,
{
    pub fn new(
        spi: Spi<'static, SPIInstance, Blocking>,
        n_chip_select: Output<'static>,
        busy: Input<'static>,
    ) -> Self {
        Radio {
            spi,
            n_chip_select,
            busy,
            x: 0,
        }
    }

    // Write to the buffer
    pub fn set_buffer(&mut self) {
        let send_buffer: [u8; 6] = [0x1a, 0, self.x, 1, 2, 3]; // Command to send, write to the buffer starting at address 0, with data (0, 1, 2, 3)
        let mut recieve_buffer: [u8; 6] = [0, 0, 0, 0, 0, 0];
        self.n_chip_select.set_low();
        // while self.busy.is_high() {
        //     println!("busy");
        //     // spin, or add a timeout
        // }
        self.spi
            .blocking_transfer(&mut recieve_buffer, &send_buffer)
            .expect("SPI transfer failed");

        println!("set: {:?}", recieve_buffer);
        self.x = self.x.wrapping_add(1);
        self.n_chip_select.set_high();
    }

    // Read from the buffer
    pub fn get_buffer(&mut self) {
        let send_buffer: [u8; 6] = [0x1b, 0, 0, 0, 0, 0]; // Command to send, read 4 bytes from the buffer starting at address 0
        let mut recieve_buffer: [u8; 6] = [0, 0, 0, 0, 0, 0];
        self.n_chip_select.set_low();
        // while self.busy.is_high() {
        //     println!("busy lol");
        //     // spin, or add a timeout
        // }
        self.spi
            .blocking_transfer(&mut recieve_buffer, &send_buffer)
            .expect("SPI transfer failed");

        println!("get: {:?}", recieve_buffer);
        self.n_chip_select.set_high();
    }

    pub fn is_busy(&mut self) -> bool {
        self.busy.is_high()
    }
}
