#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IrqMask {
    mask: u16,
}

#[allow(unused)]
impl IrqMask {
    pub const NONE: IrqMask = IrqMask { mask: 0 };
    pub const ALL: IrqMask = IrqMask { mask: 0xFFFF };
    pub const TX_DONE: IrqMask = IrqMask { mask: 1 << 0 };
    pub const RX_DONE: IrqMask = IrqMask { mask: 1 << 1 };
    pub const SYNC_WORD_VALID: IrqMask = IrqMask { mask: 1 << 2 };
    pub const SYNC_WORD_ERROR: IrqMask = IrqMask { mask: 1 << 3 };
    pub const HEADER_VALID: IrqMask = IrqMask { mask: 1 << 4 };
    pub const HEADER_ERROR: IrqMask = IrqMask { mask: 1 << 5 };
    pub const CRC_ERROR: IrqMask = IrqMask { mask: 1 << 6 };
    pub const RANGING_SLAVE_RESPONSE_DONE: IrqMask = IrqMask { mask: 1 << 7 };
    pub const RANGING_SLAVE_REQUEST_DISCARD: IrqMask = IrqMask { mask: 1 << 8 };
    pub const RANGING_MASTER_RESULT_VALID: IrqMask = IrqMask { mask: 1 << 9 };
    pub const RANGING_MASTER_TIMEOUT: IrqMask = IrqMask { mask: 1 << 10 };
    pub const RANGING_SLAVE_REQUEST_VALID: IrqMask = IrqMask { mask: 1 << 11 };
    pub const CAD_DONE: IrqMask = IrqMask { mask: 1 << 12 };
    pub const CAD_DETECTED: IrqMask = IrqMask { mask: 1 << 13 };
    pub const RX_TX_TIMEOUT: IrqMask = IrqMask { mask: 1 << 14 };
    pub const PREAMBLE_DETECTED: IrqMask = IrqMask { mask: 1 << 15 };

    pub fn new(top: u8, bottom: u8) -> IrqMask {
        let mask: u16 = top.unbounded_shl(8) as u16 + bottom as u16;
        IrqMask { mask }
    }

    /// Combine (OR) another mask's bits into this one.
    pub fn set(&self, other: IrqMask) -> Self {
        IrqMask {
            mask: self.mask | other.mask,
        }
    }

    /// Clear (AND NOT) another mask's bits from this one.
    pub fn reset(&self, other: IrqMask) -> Self {
        IrqMask {
            mask: self.mask & !other.mask,
        }
    }

    /// irqMask[15:8] — high byte, for SPI transfer.
    pub fn top(&self) -> u8 {
        (self.mask >> 8) as u8
    }

    /// irqMask[7:0] — low byte, for SPI transfer.
    pub fn bottom(&self) -> u8 {
        (self.mask & 0xFF) as u8
    }

    /// True if every flag set in `other` is also set in `self`.
    pub fn contains(&self, other: IrqMask) -> bool {
        self.mask & other.mask == other.mask
    }
}
