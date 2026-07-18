use crate::drivers::radio::opcode::OpCode;

const NOP: u8 = 0x00;

pub struct RadioConfig {
    pub modem: ModemConfig,
    pub standby: StandbyConfig,
    pub tx_buffer_base_address: u8,
    pub rx_buffer_base_address: u8,
    pub tx_params: TxParams,
    pub packet_type: PacketType,
}

impl RadioConfig {
    pub fn new_lora() -> Self {
        let modulation = LoRaModulationParams {
            spread_factor: LoraSf::LoraSf10,
            bandwidth: LoraBw::LoraBw800,
            coding_rate: LoraCr::LoraCrLi4_6,
        };
        let packet = LoRaPacketParams {
            preamble_length_mant: 3,
            preamble_length_exp: 2,
            header_type: LoraHeaderType::ExplicitHeader,
            payload_length: 1,
            crc: LoraCrc::Enable,
            iq: LoraIq::Std,
        };
        let modem = ModemConfig::LoRa { modulation, packet };

        let tx_params = TxParams {
            power: 0x1F, // Output Power (dB) = -18 + power, from 0 to 31
            ramp_time: RampTime::Ramp20Us,
        };

        RadioConfig {
            modem,
            standby: StandbyConfig::StdbyRc,
            tx_buffer_base_address: 0x00,
            rx_buffer_base_address: 0x80,
            tx_params,
            packet_type: PacketType::LORA,
        }
    }

    pub fn set_tx_params_command(&self) -> [u8; 3] {
        [
            OpCode::SetTxParams as u8,
            self.tx_params.power,
            self.tx_params.ramp_time as u8,
        ]
    }

    pub fn set_standby_command(&self) -> [u8; 2] {
        [OpCode::SetStandby as u8, self.standby as u8]
    }

    pub fn set_tx_command(&self) -> [u8; 4] {
        // No timeout, can be added, refer to datasheet
        [OpCode::SetTx as u8, 0, 0, 0]
    }

    pub fn set_rx_command(&self) -> [u8; 4] {
        // No timeout, can be added, refer to datasheet
        [OpCode::SetRx as u8, 0, 0, 0]
    }

    pub fn set_packet_type_command(&self) -> [u8; 2] {
        [OpCode::SetPacketType as u8, self.packet_type as u8]
    }

    pub fn set_buffer_base_address_command(&self) -> [u8; 3] {
        [
            OpCode::SetBufferBaseAddress as u8,
            self.tx_buffer_base_address,
            self.rx_buffer_base_address,
        ]
    }

    pub fn set_modulation_params_command(&self) -> [u8; 4] {
        match &self.modem {
            ModemConfig::LoRa { modulation, .. } => [
                OpCode::SetModulationParams as u8,
                modulation.spread_factor as u8,
                modulation.bandwidth as u8,
                modulation.coding_rate as u8,
            ],
        }
    }

    pub fn set_packet_params_command(&self) -> [u8; 8] {
        match self.modem {
            ModemConfig::LoRa { packet, .. } => {
                // Default preamble reccommended is 12
                // Preamble length = preamble_len_mant x 2 ^ preamble_len_exp
                // e.g. 3 x 2 ^ 2 = 12
                assert!(
                    packet.preamble_length_exp < 0x10,
                    "Preamble length exponent must be 4 bits or less"
                );
                assert!(
                    packet.preamble_length_mant < 0x10,
                    "Preamble length mantissa must be 4 bits or less"
                );

                // The assembled byte of preamble length
                let preamble_len =
                    packet.preamble_length_exp.unbounded_shl(4) + packet.preamble_length_mant;

                [
                    OpCode::SetPacketParams as u8,
                    preamble_len,
                    packet.header_type as u8,
                    packet.payload_length,
                    packet.crc as u8,
                    packet.iq as u8,
                    NOP,
                    NOP,
                ]
            }
        }
    }

    // Set to 2.4ghz exactly, can be changed later but this should work for testing
    pub fn set_rf_frequency_command(&self) -> [u8; 4] {
        [OpCode::SetRfFrequency as u8, 0xb8, 0x9d, 0x89]
    }
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
pub enum PacketType {
    GFSK = 0,
    LORA = 1,
    Ranging = 2,
    FLRC = 3,
    BLE = 4,
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
pub enum ModemConfig {
    LoRa {
        modulation: LoRaModulationParams,
        packet: LoRaPacketParams,
    },
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
pub enum StandbyConfig {
    StdbyRc = 0,
    StdbyXosc = 1,
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
pub enum LoraSf {
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
pub enum LoraBw {
    LoraBw1600 = 0x0A,
    LoraBw800 = 0x18,
    LoraBw400 = 0x26,
    LoraBw200 = 0x34,
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
pub enum LoraCr {
    LoraCr4_5 = 0x01,
    LoraCr4_6 = 0x02,
    LoraCr4_7 = 0x03,
    LoraCr4_8 = 0x04,
    LoraCrLi4_5 = 0x05,
    LoraCrLi4_6 = 0x06,
    LoraCrLi4_8 = 0x07,
}

#[derive(defmt::Format, Clone, Copy)]
pub struct LoRaModulationParams {
    pub spread_factor: LoraSf,
    pub bandwidth: LoraBw,
    pub coding_rate: LoraCr,
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
pub enum LoraHeaderType {
    ExplicitHeader = 0x00,
    ImplicitHeader = 0x80,
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
pub enum LoraCrc {
    Enable = 0x20,
    Disable = 0x00,
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
pub enum LoraIq {
    Inverted = 0x00,
    Std = 0x40,
}

#[derive(defmt::Format, Clone, Copy)]
pub struct LoRaPacketParams {
    preamble_length_mant: u8,
    preamble_length_exp: u8,
    header_type: LoraHeaderType,
    payload_length: u8,
    crc: LoraCrc,
    iq: LoraIq,
}

#[repr(u8)]
#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
pub enum RampTime {
    Ramp02Us = 0x00,
    Ramp04Us = 0x20,
    Ramp06Us = 0x40,
    Ramp08Us = 0x60,
    Ramp10Us = 0x80,
    Ramp12Us = 0xA0,
    Ramp16Us = 0xC0,
    Ramp20Us = 0xE0,
}

pub struct TxParams {
    power: u8,
    ramp_time: RampTime,
}
