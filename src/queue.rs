use core::{hint::spin_loop, mem, ptr::NonNull};

use tock_registers::{register_bitfields, register_structs, registers::ReadWrite};

use crate::{dma::DMAVec, err::*, registers::NvmeReg, OS};

const NVME_QUEUE_DEPTH: usize = 1024;

register_bitfields! [
    u32,
    pub CommandDword0 [
        Opcode OFFSET(0) NUMBITS(8) [],
        FusedOperation OFFSET(8) NUMBITS(2) [
            Normal = 0,
            FusedFirst = 0b1,
            FusedSecond = 0b10,
            Reserved = 0b11,
        ],
        PSDT OFFSET(14) NUMBITS(2) [
            PRP = 0,
            SGLSignal = 0b1,
            SGLExactly = 0b10,
            Reserved = 0b11,
        ],
        CommandId OFFSET(16) NUMBITS(16) []
    ],
];

#[repr(transparent)]
pub struct NvmeSubmission([u8; 64]);

pub trait Submission {
    fn to_submission(self) -> NvmeSubmission;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
//64B
pub struct AdminAndNvmCommandSetPRP {
    pub cdw0: u32,
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

impl Submission for AdminAndNvmCommandSetPRP {
    fn to_submission(self) -> NvmeSubmission {
        unsafe { mem::transmute(self) }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
struct NvmeCompletion {
    pub result: u64,
    pub sq_head: u16,
    pub sq_id: u16,
    pub command_id: u16,
    pub status: CompletionStatus,
}

#[repr(transparent)]
#[derive(Debug, Copy, Clone, Default)]
struct CompletionStatus(u16);

impl CompletionStatus {
    pub fn phase(&self) -> bool {
        self.0 & 1 > 0
    }

    fn is_success(&self) -> bool {
        self.0 & (1 << 1) == 0
    }
}

pub struct NvmeQueue<O: OS> {
    pub qid: usize,

    pub sq: SubmitQueue<O>,
    pub cq: CompleteQueue<O>,

    pub data: DMAVec<u8, O>,

    pub reg: NonNull<NvmeReg>,
}

impl<O: OS> NvmeQueue<O> {
    pub fn new(qid: usize, reg: NonNull<NvmeReg>) -> Result<Self> {
        let data = DMAVec::zeros(O::page_size() * 4)?;
        let submit_queue = SubmitQueue::new(NVME_QUEUE_DEPTH)?;
        let complete_queue = CompleteQueue::new(NVME_QUEUE_DEPTH)?;

        Ok(NvmeQueue {
            sq: submit_queue,
            cq: complete_queue,
            qid,
            data,
            reg,
        })
    }

    pub fn depth(&self) -> usize {
        NVME_QUEUE_DEPTH
    }

    fn reg(&self) -> &NvmeReg {
        unsafe { self.reg.as_ref() }
    }

    fn submit_admin_data(&mut self, data: AdminAndNvmCommandSetPRP) {
        let tail = self.sq.submit(data);
        self.reg().write_sq_y_tail_doolbell(self.qid, tail);
    }

    pub fn command_sync(&mut self, data: AdminAndNvmCommandSetPRP) -> Result<()> {
        self.submit_admin_data(data);
        let complete = self.cq.spin_for_complete();
        self.reg().write_cq_y_head_doolbell(self.qid, self.cq.head);

        if complete.status.is_success() {
            Ok(())
        } else {
            Err(Error::Unknown("send command failed"))
        }
    }
}

pub struct SubmitQueue<O: OS> {
    queue: DMAVec<NvmeSubmission, O>,
    tail: u32,
}

impl<O: OS> SubmitQueue<O> {
    fn new(queue_size: usize) -> Result<Self> {
        let queue = DMAVec::zeros(queue_size)?;
        Ok(SubmitQueue { queue, tail: 0 })
    }

    // returns the submission queue tail
    pub fn submit(&mut self, data: impl Submission) -> u32 {
        let item = &mut self.queue[self.tail as usize] as *mut NvmeSubmission;
        unsafe {
            item.write_volatile(data.to_submission());
        }
        self.tail += 1;
        if self.tail >= self.len() as u32 {
            self.tail = 0;
        }
        self.tail
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn bus_addr(&self) -> u64 {
        self.queue.bus_addr()
    }
}

pub struct CompleteQueue<O: OS> {
    queue: DMAVec<NvmeCompletion, O>,
    head: u32,
    phase: bool,
}

impl<O: OS> CompleteQueue<O> {
    fn new(queue_size: usize) -> Result<Self> {
        let queue = DMAVec::zeros(queue_size)?;
        Ok(CompleteQueue {
            queue,
            head: 0,
            phase: false,
        })
    }

    // check if there is completed command in completion queue
    fn complete(&self) -> Option<NvmeCompletion> {
        let cqe = unsafe { self.queue.as_ptr().add(self.head as _).read_volatile() };
        let complete = cqe.status.phase() != self.phase;

        if complete {
            Some(cqe)
        } else {
            None
        }
    }

    fn spin_for_complete(&mut self) -> NvmeCompletion {
        loop {
            if let Some(e) = self.complete() {
                let next_head = self.head + 1;
                if next_head >= self.queue.len() as u32 {
                    self.head = 0;
                    self.phase ^= self.phase;
                } else {
                    self.head = next_head;
                }

                return e;
            }
            spin_loop();
        }
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn bus_addr(&self) -> u64 {
        self.queue.bus_addr()
    }
}
