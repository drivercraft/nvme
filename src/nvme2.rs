use core::ptr::NonNull;

use log::{debug, info};

use crate::{
    command::Feature,
    err::*,
    queue::{CommandSet, NvmeQueue},
    registers::NvmeReg,
    OS,
};

pub struct Nvme<O: OS> {
    bar: NonNull<NvmeReg>,
    admin_queue: NvmeQueue<O>,
    io_queues: NvmeQueue<O>,
}

impl<O: OS> Nvme<O> {
    pub fn new(bar: NonNull<u8>) -> Result<Self> {
        let admin_queue = NvmeQueue::new(0, bar.cast())?;
        let io_queues = NvmeQueue::new(1, bar.cast())?;

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

        s.config_io_queue()?;

        debug!("IO queue ok.");

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

    fn config_io_queue(&mut self) -> Result {
        // 设置 io queue 数量
        let cmd = CommandSet::set_features(Feature::NumberOfQueues {
            nsq: self.io_queues.sq.len() as u32 - 1,
            ncq: self.io_queues.cq.len() as u32 - 1,
        });
        self.admin_queue.command_sync(cmd)?;

        let data = CommandSet::create_io_completion_queue(
            self.io_queues.qid,
            self.io_queues.cq.len() as _,
            self.io_queues.cq.bus_addr(),
            true,
            false,
            0,
        );

        self.admin_queue.command_sync(data)?;

        let data = CommandSet::create_io_submission_queue(
            self.io_queues.qid,
            self.io_queues.sq.len() as _,
            self.io_queues.sq.bus_addr(),
            true,
            0,
            self.io_queues.qid,
            0,
        );

        self.admin_queue.command_sync(data)?;

        Ok(())
    }

    pub fn version(&self) -> (usize, usize, usize) {
        self.reg().version()
    }

    fn reg(&self) -> &NvmeReg {
        unsafe { self.bar.as_ref() }
    }
}
