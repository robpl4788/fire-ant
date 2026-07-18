use crate::drivers::radio::radio_config::ModemConfig;

#[derive(defmt::Format)]
enum RadioMode {
    Reserved,
    StdbyRc,
    StdbyXosc,
    FS,
    Rx,
    Tx,
    Error,
}

#[derive(defmt::Format)]
enum RadioCommandStatus {
    Reserved,
    CommandSuccess,
    DataAvailable,
    CommandTimeout,
    CommandError,
    FailedToExecuteCommand,
    CommandTxDone,
    Error,
}

#[derive(defmt::Format)]
pub struct RadioStatus {
    mode: RadioMode,
    command_status: RadioCommandStatus,
}

impl RadioStatus {
    pub fn new() -> Self {
        Self {
            mode: RadioMode::Error,
            command_status: RadioCommandStatus::Error,
        }
    }

    pub fn interpret_status(status: &u8) -> Self {
        let mode_raw = status.unbounded_shr(5);
        let command_status_raw = status.unbounded_shr(2) % 8;
        let mode = match mode_raw {
            0 => RadioMode::Reserved,
            1 => RadioMode::Reserved,
            2 => RadioMode::StdbyRc,
            3 => RadioMode::StdbyXosc,
            4 => RadioMode::FS,
            5 => RadioMode::Rx,
            6 => RadioMode::Tx,
            _ => RadioMode::Error,
        };

        let command_status = match command_status_raw {
            0 => RadioCommandStatus::Reserved,
            1 => RadioCommandStatus::CommandSuccess,
            2 => RadioCommandStatus::DataAvailable,
            3 => RadioCommandStatus::CommandTimeout,
            4 => RadioCommandStatus::CommandError,
            5 => RadioCommandStatus::FailedToExecuteCommand,
            6 => RadioCommandStatus::CommandTxDone,
            _ => RadioCommandStatus::Error,
        };

        Self {
            mode,
            command_status,
        }
    }
}

#[derive(defmt::Format, Clone, Copy)]
#[allow(unused)]
pub enum PacketStatus {
    Lora { rssi_sync_raw: u8, snr_pkt_raw: u8 },
}

impl PacketStatus {
    pub fn interpret_status(raw_status: [u8; 7], modem: ModemConfig) -> PacketStatus {
        match modem {
            ModemConfig::LoRa { .. } => PacketStatus::Lora {
                rssi_sync_raw: raw_status[2],
                snr_pkt_raw: raw_status[3],
            },
        }
    }
}
