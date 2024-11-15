use core::{marker::PhantomData, ptr::NonNull};

use log::{debug, info};

use crate::{err::*, queue::NvmeQueue, registers::NvmeReg, OS};

pub struct Nvme<O: OS> {
    bar: NonNull<NvmeReg>,
    admin_queue: NvmeQueue<O>,
    io_queues: NvmeQueue<O>,
}

impl<O: OS> Nvme<O> {
    pub fn new(bar: NonNull<u8>) -> Result<Self> {
        let admin_queue = NvmeQueue::new(0, 0)?;
        let io_queues = NvmeQueue::new(1, 0x8)?;

        let mut s = Self {
            bar: bar.cast(),
            admin_queue,
            io_queues,
        };

        let version = s.version();

        info!(
            "NVME @{bar:?} init begin, version: {}.{}.{} ",
            version.0, version.1, version.2
        );

        s.nvme_configure_admin_queue();

        Ok(s)
    }

    // config admin queue
    // 1. set admin queue(cq && sq) size
    // 2. set admin queue(cq && sq) dma address
    // 3. enable ctrl
    fn nvme_configure_admin_queue(&mut self) {
        self.reg().set_admin_submission_and_completion_queue_size(
            self.admin_queue.sq.len(),
            self.admin_queue.cq.len(),
        );

        self.reg()
            .set_admin_submission_queue_base_address(self.admin_queue.sq.bus_addr());

        self.reg()
            .set_admin_completion_queue_base_address(self.admin_queue.cq.bus_addr());


        self.reg().enable_ctrl();

        debug!("Enabled ctrl");
    }

    pub fn version(&self) -> (usize, usize, usize) {
        self.reg().version()
    }

    fn reg(&self) -> &NvmeReg {
        unsafe { self.bar.as_ref() }
    }
}
