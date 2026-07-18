use defmt::println;
use embassy_rp::{
    gpio::{Input, Output},
    spi::{self, Blocking, Spi},
};

const NOP: u8 = 0x00;

use crate::drivers::radio::status::{PacketStatus, RadioStatus};
use crate::drivers::radio::{
    irq::IrqMask, opcode::OpCode, radio_config::LoraSf, radio_config::ModemConfig,
    radio_config::RadioConfig,
};

#[derive(defmt::Format, Clone, Copy)]
struct RxBufferStatus {
    rx_payload_length: u8,
    rx_start_buffer_pointer: u8,
}

pub struct Radio<SPIInstance>
where
    SPIInstance: spi::Instance + 'static,
{
    spi: Spi<'static, SPIInstance, Blocking>,
    n_chip_select: Output<'static>,
    busy: Input<'static>,
    tx_done: Input<'static>,
    rx_done: Input<'static>,
    _dio3: Input<'static>,
    status: RadioStatus,
    config: RadioConfig,
}

#[allow(dead_code)]
impl<SPIInstance> Radio<SPIInstance>
where
    SPIInstance: spi::Instance + 'static,
{
    pub fn new(
        spi: Spi<'static, SPIInstance, Blocking>,
        n_chip_select: Output<'static>,
        busy: Input<'static>,
        dio1: Input<'static>,
        dio2: Input<'static>,
        dio3: Input<'static>,
    ) -> Self {
        let mut radio = Radio {
            spi,
            n_chip_select,
            busy,
            status: RadioStatus::new(),
            tx_done: dio1,
            rx_done: dio2,
            _dio3: dio3,
            config: RadioConfig::new_lora(),
        };

        radio.start_lora();

        radio
    }

    fn start_lora(&mut self) {
        self.set_standby();
        self.set_packet_type();
        self.set_rf_frequency();
        self.set_buffer_base_address();
        self.set_lora_modulation_params();
        self.set_packet_params();
        self.set_dio_irq_params();
        self.set_tx_params();
    }

    pub fn transmit(&mut self, data: u8) {
        self.clear_all_irq_status();
        self.write_buffer_single(self.config.tx_buffer_base_address, data);
        self.set_tx();
    }

    pub async fn recieve(&mut self) -> u8 {
        self.set_rx();
        self.rx_done.wait_for_high().await;

        let packet_status = self.get_packet_status();

        println!("packet_status: {}", packet_status);

        let irq_status = self.get_irq_status();

        if irq_status != IrqMask::RX_DONE {
            println!("irq status error: {}", irq_status);
        }

        self.clear_all_irq_status();

        let rx_buffer_status = self.get_rx_buffer_status();
        println!("rx_buffer_status: {}", rx_buffer_status);

        self.read_buffer_single(rx_buffer_status.rx_start_buffer_pointer)
    }

    pub fn set_rx(&mut self) {
        self.spi_command(&mut self.config.set_rx_command());
    }

    fn get_packet_status(&mut self) -> PacketStatus {
        let mut command = [OpCode::GetPacketStatus as u8, NOP, NOP, NOP, NOP, NOP, NOP];
        self.spi_command(&mut command);

        PacketStatus::interpret_status(command, self.config.modem)
    }
    fn get_rx_buffer_status(&mut self) -> RxBufferStatus {
        let mut command = [OpCode::GetRxBufferStatus as u8, NOP, NOP, NOP];
        self.spi_command(&mut command);

        RxBufferStatus {
            rx_payload_length: command[2],
            rx_start_buffer_pointer: command[3],
        }
    }

    fn set_standby(&mut self) {
        self.spi_command(&mut self.config.set_standby_command());
    }

    fn set_tx(&mut self) {
        self.spi_command(&mut self.config.set_tx_command());
    }

    fn set_packet_type(&mut self) {
        self.spi_command(&mut self.config.set_packet_type_command());
    }

    fn set_rf_frequency(&mut self) {
        self.spi_command(&mut self.config.set_rf_frequency_command());
    }

    // Set the buffer base addresses to be constants defined above
    fn set_buffer_base_address(&mut self) {
        self.spi_command(&mut self.config.set_buffer_base_address_command());
    }

    fn set_lora_modulation_params(&mut self) {
        self.spi_command(&mut self.config.set_modulation_params_command());

        // Add corrections for lora
        #[allow(irrefutable_let_patterns)]
        if let ModemConfig::LoRa { modulation, .. } = self.config.modem {
            match modulation.spread_factor {
                LoraSf::LoraSf5 | LoraSf::LoraSf6 => {
                    self.write_single_register(0x925, 0x1E);
                }
                LoraSf::LoraSf7 | LoraSf::LoraSf8 => {
                    self.write_single_register(0x925, 0x37);
                }
                LoraSf::LoraSf9 | LoraSf::LoraSf10 | LoraSf::LoraSf11 | LoraSf::LoraSf12 => {
                    self.write_single_register(0x925, 0x32);
                }
            }

            self.write_single_register(0x93c, 0x1); // Frequency Error Compensation Register
        }
    }

    fn set_packet_params(&mut self) {
        // Can also set the Synch word but isn't implemented atm, see sx1280 datasheet v3.3 page 133
        self.spi_command(&mut self.config.set_packet_params_command());
    }

    fn set_tx_params(&mut self) {
        let mut command = self.config.set_tx_params_command();

        // Can also set the Synch word but isn't implemented atm, see sx1280 datasheet v3.3 page 133
        self.spi_command(&mut command);
    }

    fn set_dio_irq_params(&mut self) {
        let dio_1_mask = IrqMask::TX_DONE;
        let dio_2_mask = IrqMask::RX_DONE;
        let dio_3_mask = IrqMask::RX_TX_TIMEOUT;

        let irq_mask = dio_1_mask
            .set(dio_2_mask)
            .set(dio_3_mask)
            .set(IrqMask::HEADER_ERROR)
            .set(IrqMask::CRC_ERROR);

        let mut command = [
            OpCode::SetDioIrqParams as u8,
            irq_mask.top(),
            irq_mask.bottom(),
            dio_1_mask.top(),
            dio_1_mask.bottom(),
            dio_2_mask.top(),
            dio_2_mask.bottom(),
            dio_3_mask.top(),
            dio_3_mask.bottom(),
        ];

        self.spi_command(&mut command);
    }

    fn get_irq_status(&mut self) -> IrqMask {
        let mut command = [OpCode::GetIrqStatus as u8, NOP, NOP, NOP];
        self.spi_command(&mut command);

        let irq_status_top = command[2];
        let irq_status_bottom = command[3];

        IrqMask::new(irq_status_top, irq_status_bottom)
    }

    fn clear_irq_status(&mut self, to_clear: IrqMask) {
        let mut command = [
            OpCode::ClrIrqStatus as u8,
            to_clear.top(),
            to_clear.bottom(),
        ];
        self.spi_command(&mut command);
    }

    fn clear_all_irq_status(&mut self) {
        self.clear_irq_status(IrqMask::ALL);
    }

    fn write_single_register(&mut self, register: u16, value: u8) {
        let [register_top, register_bottom] = register.to_be_bytes();
        let mut command = [
            OpCode::WriteRegister as u8,
            register_top,
            register_bottom,
            value,
        ];
        self.spi_command(&mut command);
    }

    fn read_single_register(&mut self, register: u16) -> u8 {
        let [register_top, register_bottom] = register.to_be_bytes();
        let mut command = [
            OpCode::ReadRegister as u8,
            register_top,
            register_bottom,
            NOP,
            NOP,
        ];
        self.spi_command(&mut command);

        command
            .last()
            .expect("Command Array should have none zero size")
            .clone()
    }

    fn spi_command(&mut self, buffer: &mut [u8]) {
        // Wait for busy to go low so the transciever is ready to recieve a command
        while self.is_busy() {
            println!("busy");
        }

        self.n_chip_select.set_low();

        self.spi
            .blocking_transfer_in_place(buffer)
            .expect("SPI transfer failed");

        self.n_chip_select.set_high();
        self.status = RadioStatus::interpret_status(&buffer[0]);

        println!("status: {:?}", &self.status);
    }

    // Write to the buffer
    pub fn write_buffer_single(&mut self, address: u8, data: u8) {
        let mut command: [u8; 3] = [OpCode::WriteBuffer as u8, address, data]; // Command to send, write to the buffer starting at address 0, with data (0, 1, 2, 3)

        self.spi_command(&mut command);
    }

    // Read from the buffer
    pub fn read_buffer_single(&mut self, address: u8) -> u8 {
        let mut command: [u8; 4] = [OpCode::ReadBuffer as u8, address, NOP, NOP]; // Command to send, read 4 bytes from the buffer starting at address 0
        self.spi_command(&mut command);
        // println!("get: {:?}", buffer);

        *command.last().expect("Command must have none zero length")
    }

    pub fn is_busy(&mut self) -> bool {
        self.busy.is_high()
    }

    pub fn is_tx_done(&mut self) -> bool {
        self.tx_done.is_high()
    }

    pub fn is_rx_done(&mut self) -> bool {
        self.rx_done.is_high()
    }
}
