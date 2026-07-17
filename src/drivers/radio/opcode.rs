#[repr(u8)]
pub enum OpCode {
    // Status
    GetStatus = 0xC0,

    // Register / buffer access
    WriteRegister = 0x18,
    ReadRegister = 0x19,
    WriteBuffer = 0x1A,
    ReadBuffer = 0x1B,

    // Mode setting
    SetSleep = 0x84,
    SetStandby = 0x80,
    SetFs = 0xC1,
    SetTx = 0x83,
    SetRx = 0x82,
    SetRxDutyCycle = 0x94,
    SetCad = 0xC5,
    SetTxContinuousWave = 0xD1,
    SetTxContinuousPreamble = 0xD2,

    // Packet / radio config
    SetPacketType = 0x8A,
    GetPacketType = 0x03,
    SetRfFrequency = 0x86,
    SetTxParams = 0x8E,
    SetCadParams = 0x88,
    SetBufferBaseAddress = 0x8F,
    SetModulationParams = 0x8B,
    SetPacketParams = 0x8C,

    // Status / results
    GetRxBufferStatus = 0x17,
    GetPacketStatus = 0x1D,
    GetRssiInst = 0x1F,

    // IRQ
    SetDioIrqParams = 0x8D,
    GetIrqStatus = 0x15,
    ClrIrqStatus = 0x97,

    // Misc
    SetRegulatorMode = 0x96,
    SetSaveContext = 0xD5,
    SetAutoFS = 0x9E,
    SetAutoTx = 0x98,
    SetLongPreamble = 0x9B,
    SetUartSpeed = 0x9D,
    SetRangingRole = 0xA3,
    SetAdvancedRanging = 0x9A,
}
