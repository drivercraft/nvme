#![allow(unused)]

#[repr(transparent)]
pub struct Opcode(u8);

impl Opcode {
    const fn new(generic: u8, function: u8, data_transfer: u8) -> Self {
        Opcode(generic << 7 | function << 2 | data_transfer)
    }

    pub fn as_u32(&self) -> u32 {
        self.0 as _
    }

    pub const DELETE_IO_SQ: Self = Self::new(0b0, 0b0, 0b0);
    pub const CREATE_IO_SQ: Self = Self::new(0b0, 0b0, 0b1);
    pub const GET_LOG_PAGE: Self = Self::new(0b0, 0b0, 0b10);
    pub const DELETE_IO_CQ: Self = Self::new(0b0, 0b1, 0b0);
    pub const CREATE_IO_CQ: Self = Self::new(0b0, 0b1, 0b1);
    pub const IDENTIFY: Self = Self::new(0b0, 0b1, 0b10);
    pub const ABORT: Self = Self::new(0b0, 0b10, 0b0);
    pub const SET_FEATURES: Self = Self::new(0b0, 0b10, 0b1);
    pub const GET_FEATURES: Self = Self::new(0b0, 0b10, 0b10);
    pub const ASYNCHRONOUS_EVENT_REQUEST: Self = Self::new(0b0, 0b11, 0b0);
    pub const NAMESPACE_MANAGEMENT: Self = Self::new(0b0, 0b11, 0b1);
    pub const FIRMWARE_COMMIT: Self = Self::new(0b1, 0b100, 0b0);
    pub const FIRMWARE_IMAGE_DOWNLOAD: Self = Self::new(0b1, 0b100, 0b1);
    pub const DEVICE_SELF_TEST: Self = Self::new(0b1, 0b101, 0b0);
    pub const NAMESPACE_ATTACHMENT: Self = Self::new(0b1, 0b101, 0b1);
    pub const KEEP_ALIVE: Self = Self::new(0b1, 0b110, 0b0);
    pub const DIRECTIVE_SEND: Self = Self::new(0b1, 0b110, 0b1);
    pub const DIRECTIVE_RECEIVE: Self = Self::new(0b1, 0b110, 0b10);
    pub const VIRTUALIZATION_MANAGEMENT: Self = Self::new(0b1, 0b111, 0b0);
    pub const NVME_MI_SEND: Self = Self::new(0b1, 0b111, 0b1);
    pub const NVME_MI_RECEIVE: Self = Self::new(0b1, 0b111, 0b10);
    pub const DOORBELL_BUFFER_CONFIG: Self = Self::new(0b111, 0b11111, 0b0);
}

pub enum Feature {
    NumberOfQueues { nsq: u32, ncq: u32 },
}

impl Feature {
    pub fn to_cdw10(&self) -> u32 {
        match self {
            Feature::NumberOfQueues { .. } => 0x7,
        }
    }
}
