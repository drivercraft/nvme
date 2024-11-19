#![no_std]

extern crate alloc;

mod command;
mod dma;
pub mod err;
mod nvme2;
mod queue;
mod registers;

use core::{alloc::Layout, ptr::NonNull, time::Duration};

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
