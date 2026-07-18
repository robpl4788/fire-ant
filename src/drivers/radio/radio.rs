use defmt::println;
use embassy_rp::{
    gpio::{Input, Output},
    pac::pio::vals::ExecctrlStatusSel::IRQ,
    spi::{self, Blocking, Spi},
};

const NOP: u8 = 0x00;

use crate::drivers::radio::irq::IrqMask;
use crate::drivers::radio::opcode::OpCode;
use crate::drivers::radio::status::RadioStatus;

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
enum PacketType {
    GFSK = 0,
    LORA = 1,
    Ranging = 2,
    FLRC = 3,
    BLE = 4,
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
enum StandbyConfig {
    StdbyRc = 0,
    StdbyXosc = 1,
}

#[repr(u8)]
enum BufferAddresses {
    TxBase = 0x80,
    RxBase = 0x00,
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
enum LoraSf {
    LoraSf5 = 0x50,
    LoraSf6 = 0x60,
    LoraSf7 = 0x70,
    LoraSf8 = 0x80,
    LoraSf9 = 0x90,
    LoraSf10 = 0xa0,
    LoraSf11 = 0xb0,
    LoraSf12 = 0xc0,
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
enum LoraBw {
    LoraBw1600 = 0x0A,
    LoraBw800 = 0x18,
    LoraBw400 = 0x26,
    LoraBw200 = 0x34,
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
enum LoraCr {
    LoraCr4_5 = 0x01,
    LoraCr4_6 = 0x02,
    LoraCr4_7 = 0x03,
    LoraCr4_8 = 0x04,
    LoraCrLi4_5 = 0x05,
    LoraCrLi4_6 = 0x06,
    LoraCrLi4_8 = 0x07,
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
enum LoraHeaderType {
    ExplicitHeader = 0x00,
    ImplicitHeader = 0x80,
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
enum LoraCrc {
    Enable = 0x20,
    Disable = 0x00,
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
enum LoraIq {
    Inverted = 0x00,
    Std = 0x40,
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
enum RampTime {
    Ramp02Us = 0x00,
    Ramp04Us = 0x20,
    Ramp06Us = 0x40,
    Ramp08Us = 0x60,
    Ramp10Us = 0x80,
    Ramp12Us = 0xA0,
    Ramp16Us = 0xC0,
    Ramp20Us = 0xE0,
}

#[derive(defmt::Format, Clone, Copy)]
struct LoraPacketStatus {
    rssi_sync_raw: u8,
    snr_pkt_raw: u8,
}

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
    _rx_done: Input<'static>,
    _dio3: Input<'static>,
    status: RadioStatus,
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
            _rx_done: dio2,
            _dio3: dio3,
        };

        radio.start_lora();

        radio
    }

    fn start_lora(&mut self) {
        self.set_standby(StandbyConfig::StdbyXosc);
        self.set_packet_type(PacketType::LORA);
        self.set_rf_frequency();
        self.set_buffer_base_address();
        self.set_lora_modulation_params(LoraSf::LoraSf10, LoraBw::LoraBw800, LoraCr::LoraCrLi4_6);
        self.set_packet_params();
        self.set_dio_irq_params();
        self.set_tx_params();
    }

    pub fn transmit(&mut self, data: u8) {
        self.clear_all_irq_status();
        self.write_buffer_single(BufferAddresses::TxBase as u8, data);
        self.set_tx();
    }

    pub async fn recieve(&mut self) -> u8 {
        self.set_rx();
        self._rx_done.wait_for_high().await;

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
        // No timeout, can be added, refer to datasheet
        let mut command = [OpCode::SetRx as u8, 0, 0, 0];
        self.spi_command(&mut command);
    }

    fn get_packet_status(&mut self) -> LoraPacketStatus {
        let mut command = [OpCode::GetPacketStatus as u8, NOP, NOP, NOP, NOP, NOP, NOP];
        self.spi_command(&mut command);

        LoraPacketStatus {
            rssi_sync_raw: command[2],
            snr_pkt_raw: command[3],
        }
    }
    fn get_rx_buffer_status(&mut self) -> RxBufferStatus {
        let mut command = [OpCode::GetRxBufferStatus as u8, NOP, NOP, NOP];
        self.spi_command(&mut command);

        RxBufferStatus {
            rx_payload_length: command[2],
            rx_start_buffer_pointer: command[3],
        }
    }

    fn set_standby(&mut self, standby_config: StandbyConfig) {
        let mut command = [OpCode::SetStandby as u8, standby_config as u8];
        self.spi_command(&mut command);
    }

    fn set_tx(&mut self) {
        // No timeout, can be added, refer to datasheet
        let mut command = [OpCode::SetTx as u8, 0, 0, 0];
        self.spi_command(&mut command);
    }

    fn set_packet_type(&mut self, packet_type: PacketType) {
        let mut command = [OpCode::SetPacketType as u8, packet_type as u8];
        self.spi_command(&mut command);
    }

    // Set to 2.4ghz exactly, can be changed later but this should work for testing
    fn set_rf_frequency(&mut self) {
        let mut command = [OpCode::SetRfFrequency as u8, 0xb8, 0x9d, 0x89];
        self.spi_command(&mut command);
    }

    // Set the buffer base addresses to be constants defined above
    fn set_buffer_base_address(&mut self) {
        let mut command = [
            OpCode::SetBufferBaseAddress as u8,
            BufferAddresses::TxBase as u8,
            BufferAddresses::RxBase as u8,
        ];
        self.spi_command(&mut command);
    }

    fn set_lora_modulation_params(
        &mut self,
        spreading_factor: LoraSf,
        bandwidth: LoraBw,
        coding_rate: LoraCr,
    ) {
        let mut command = [
            OpCode::SetModulationParams as u8,
            spreading_factor.clone() as u8,
            bandwidth as u8,
            coding_rate as u8,
        ];
        self.spi_command(&mut command);

        match spreading_factor {
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

    fn set_packet_params(&mut self) {
        // Default preamble reccommended is 12
        // Preamble length = preamble_len_mant x 2 ^ preamble_len_exp
        // e.g. 3 x 2 ^ 2 = 12
        let preamble_len_mant: u8 = 3;
        let preamble_len_exp: u8 = 2;

        // The assembled byte of preamble length
        let preamble_len = preamble_len_exp.unbounded_shl(4) + preamble_len_mant;

        let payload_length: u8 = 1;

        let mut command = [
            OpCode::SetPacketParams as u8,
            preamble_len,
            LoraHeaderType::ExplicitHeader as u8,
            payload_length,
            LoraCrc::Enable as u8,
            LoraIq::Std as u8,
            NOP,
            NOP,
        ];

        // Can also set the Synch word but isn't implemented atm, see sx1280 datasheet v3.3 page 133
        self.spi_command(&mut command);
    }

    fn set_tx_params(&mut self) {
        let power: u8 = 0x1f; // Output Power (dB) = -18 + power, from 0 to 31

        let mut command = [OpCode::SetTxParams as u8, power, RampTime::Ramp20Us as u8];

        // Can also set the Synch word but isn't implemented atm, see sx1280 datasheet v3.3 page 133
        self.spi_command(&mut command);
    }

    fn set_dio_irq_params(&mut self) {
        let dio_1_mask = IrqMask::TX_DONE;
        let dio_2_mask = IrqMask::RX_DONE;
        let dio_3_mask = IrqMask::NONE;

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
        self._rx_done.is_high()
    }
}
