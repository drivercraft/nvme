use crate::{err::*, DMAVec, OS};

const NVME_QUEUE_DEPTH: usize = 1024;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
//64B
pub struct NvmeSubmission {
    pub opcode: u8,
    pub flags: u8,
    pub command_id: u16,
    pub nsid: u32,
    pub cdw2: [u32; 2],
    pub metadata: u64,
    pub prp1: u64,
    pub prp2: u64,
    pub cdw10: u32,
    pub cdw11: u32,
    pub cdw12: u32,
    pub cdw13: u32,
    pub cdw14: u32,
    pub cdw15: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct NvmeCompletion {
    pub result: u64,
    pub sq_head: u16,
    pub sq_id: u16,
    pub command_id: u16,
    pub status: u16,
}

pub struct NvmeQueue<O: OS> {
    pub qid: usize,

    pub sq: DMAVec<NvmeSubmission, O>,
    pub cq: DMAVec<NvmeCompletion, O>,

    // doorbell register offset of bar address
    pub db_offset: usize,

    pub cq_head: usize,
    pub cq_phase: usize,

    pub sq_tail: usize,
    pub last_sq_tail: usize,

    pub data: DMAVec<u8, O>,
}

impl<O: OS> NvmeQueue<O> {
    pub fn new(qid: usize, db_offset: usize) -> Result<Self> {
        let data = DMAVec::zeros(O::page_size() * 4)?;
        let submit_queue = DMAVec::zeros(NVME_QUEUE_DEPTH)?;
        let complete_queue = DMAVec::zeros(NVME_QUEUE_DEPTH)?;

        Ok(NvmeQueue {
            sq: submit_queue,
            cq: complete_queue,
            db_offset,
            qid,
            cq_head: 0,
            cq_phase: 1,
            sq_tail: 0,
            last_sq_tail: 0,
            data,
        })
    }

    pub fn depth(&self) -> usize {
        NVME_QUEUE_DEPTH
    }

    pub fn nvme_init_queue(&mut self) {
        self.cq_head = 0;
        self.cq_phase = 1;
        self.sq_tail = 0;
        self.last_sq_tail = 0;
    }
}
