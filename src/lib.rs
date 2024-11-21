#![no_std]

extern crate alloc;

mod command;
pub mod err;
mod nvme;
mod queue;
mod registers;

use core::{alloc::Layout, ptr::NonNull};

pub use dma_api::{set_impl, Impl};
pub use nvme::Nvme;

#[derive(Clone, Copy)]
pub struct DMAMem {
    pub virt: NonNull<u8>,
    pub phys: u64,
    pub layout: Layout,
}
