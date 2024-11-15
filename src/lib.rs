#![no_std]

extern crate alloc;

mod dma;
mod irq;
mod nvme;

pub mod err;
mod nvme2;
mod queue;
mod registers;

use core::alloc::Layout;
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::time::Duration;

pub use dma::*;
pub use irq::*;
use log::info;
pub use nvme::*;
use registers::NvmeReg;

pub use self::dma::DmaAllocator;
pub use self::irq::IrqController;
pub use self::nvme::NvmeInterface;
pub use nvme2::Nvme;

pub trait OS {
    fn dma_alloc(layout: Layout) -> Option<DMAMem>;

    fn dma_dealloc(dma: DMAMem);

    fn sleep(duration: Duration);

    fn page_size() -> usize;
}

#[derive(Clone, Copy)]
pub struct DMAMem {
    pub virt: NonNull<u8>,
    pub phys: u64,
    pub layout: Layout,
}
