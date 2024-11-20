use core::ptr::NonNull;

use alloc::vec::Vec;
use log::{debug, info};

use crate::{
    command::{
        self, Feature, Identify, IdentifyActiveNamespaceList, IdentifyController,
        IdentifyNamespaceDataStructure,
    },
    dma::DMAVec,
    err::*,
    queue::{CommandSet, NvmeQueue},
    registers::NvmeReg,
    OS,
};

pub struct Nvme<O: OS> {
    bar: NonNull<NvmeReg>,
    admin_queue: NvmeQueue<O>,
    io_queues: NvmeQueue<O>,
    num_ns: usize,
}

impl<O: OS> Nvme<O> {
    pub fn new(bar: NonNull<u8>) -> Result<Self> {
        let admin_queue = NvmeQueue::new(0, bar.cast(), 64, 64)?;
        let io_queues = NvmeQueue::new(1, bar.cast(), 6, 4)?;

        let mut s = Self {
            bar: bar.cast(),
            admin_queue,
            io_queues,
            num_ns: 0,
        };

        let version = s.version();

        info!(
            "NVME @{bar:?} init begin, version: {}.{}.{} ",
            version.0, version.1, version.2
        );

        s.init()?;

        Ok(s)
    }

    fn init(&mut self) -> Result {
        self.reg().reset();

        self.nvme_configure_admin_queue();

        self.reg().setup_controller_settings();

        let controller = self.get_identfy(IdentifyController::new())?;

        debug!("Controller: {:?}", controller);

        self.num_ns = controller.number_of_namespaces as _;

        self.config_io_queue()?;

        debug!("IO queue ok.");
        loop {
            let ns = self.get_identfy(IdentifyNamespaceDataStructure::new(1))?;
            if let Some(ns) = ns {
                debug!("Namespace: {:?}", ns);
                break;
            }
        }
        debug!("Namespace ok.");
        Ok(())
    }
    // pub fn namespace_list(&mut self) -> Result<Vec<Namespace>> {
    //     let mut out = Vec::new();

    //     for i in 0..self.num_ns {
    //         let id = i as u32 + 1;

    //         let ns = self.get_identfy(IdentifyNamespaceDataStructure::new(id))?;

    //         if let Some(ns) = ns {
    //             out.push(Namespace {
    //                 id,
    //                 lba_size: ns.lba_size as _,
    //                 lba_count: ns.namespace_size as _,
    //                 metadata_size: ns.metadata_size as _,
    //             });
    //         }
    //     }

    //     Ok(out)
    // }
    pub fn namespace_list(&mut self) -> Result<Vec<Namespace>> {
        let id_list = self.get_identfy(IdentifyActiveNamespaceList::new())?;
        let mut out = Vec::new();

        for id in id_list {
            let ns = self
                .get_identfy(IdentifyNamespaceDataStructure::new(id))?
                .unwrap();

            out.push(Namespace {
                id,
                lba_size: ns.lba_size as _,
                lba_count: ns.namespace_size as _,
                metadata_size: ns.metadata_size as _,
            });
        }

        Ok(out)
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

    pub fn get_identfy<T: Identify>(&mut self, mut want: T) -> Result<T::Output> {
        let cmd = want.command_set_mut();

        cmd.cdw0 = CommandSet::cdw0_from_opcode(command::Opcode::IDENTIFY);
        cmd.cdw10 = T::CNS;

        let data = DMAVec::<u8, O>::zeros(0x1000)?;
        cmd.prp1 = data.bus_addr();

        self.admin_queue.command_sync(*cmd)?;

        let res = want.parse(&data);

        Ok(res)
    }

    pub fn block_write_sync(
        &mut self,
        ns: &Namespace,
        block_start: u64,
        buff: &[u8],
    ) -> Result<()> {
        assert!(
            buff.len() % ns.lba_size == 0,
            "buffer size must be multiple of lba size"
        );

        let mut data = DMAVec::<u8, O>::zeros(buff.len())?;

        data.copy_from_slice(buff);

        let blk_num = buff.len() / ns.lba_size;

        let cmd = CommandSet::nvm_cmd_write(ns.id, data.bus_addr(), block_start, blk_num as _);

        self.io_queues.command_sync(cmd)?;

        Ok(())
    }

    pub fn block_read_sync(
        &mut self,
        ns: &Namespace,
        block_start: u64,
        buff: &mut [u8],
    ) -> Result<()> {
        assert!(
            buff.len() % ns.lba_size == 0,
            "buffer size must be multiple of lba size"
        );

        let data = DMAVec::<u8, O>::zeros(buff.len())?;

        let blk_num = buff.len() / ns.lba_size;

        let cmd = CommandSet::nvm_cmd_read(ns.id, data.bus_addr(), block_start, blk_num as _);

        self.io_queues.command_sync(cmd)?;

        buff.copy_from_slice(&data);

        Ok(())
    }

    pub fn version(&self) -> (usize, usize, usize) {
        self.reg().version()
    }

    fn reg(&self) -> &NvmeReg {
        unsafe { self.bar.as_ref() }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Namespace {
    pub id: u32,
    pub lba_size: usize,
    pub lba_count: usize,
    pub metadata_size: usize,
}
