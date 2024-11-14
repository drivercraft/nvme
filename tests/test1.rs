#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(bare_test::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
extern crate bare_test;

#[bare_test::entry]
fn main() {
    test_main();
}

use bare_test::{driver::device_tree::get_device_tree, mem::mmu::iomap, println};
use log::*;
use nvme_driver::*;
use pcie::preludes::*;
use pcie::PciDevice;

#[test_case]
fn test_uart() {
    let fdt = get_device_tree().unwrap();
    let pcie = fdt
        .find_compatible(&["pci-host-ecam-generic"])
        .next()
        .unwrap();
    let mut pcie_regs = alloc::vec![];

    println!("test nvme");

    println!("pcie: {}", pcie.name);

    for reg in pcie.reg().unwrap() {
        println!("pcie reg: {:#x}", reg.address);
        pcie_regs.push(iomap((reg.address as usize).into(), reg.size.unwrap()));
    }

    let base_vaddr = pcie_regs[0];

    info!("Init PCIE @{:?}", base_vaddr);

    let mut root = pcie::RootGeneric::new(base_vaddr.as_ptr() as usize);

    root.enumerate().for_each(|device| {
        let address = device.address();
        debug!("PCI {}", device);

        if let PciDevice::Endpoint(mut ep) = device {
            if ep.device_type() == DeviceType::NvmeController {
                ep.update_command(|cmd| {
                    cmd | CommandRegister::IO_ENABLE
                        | CommandRegister::MEMORY_ENABLE
                        | CommandRegister::BUS_MASTER_ENABLE
                });
            }
        }
    });

    println!("test passed!");
}

pub struct IrqProvider;

impl IrqController for IrqProvider {
    fn enable_irq(irq: usize) {}

    fn disable_irq(irq: usize) {}
}

pub struct DmaProvider;

impl DmaAllocator for DmaProvider {
    fn dma_alloc(size: usize) -> usize {
        0
    }

    fn dma_dealloc(addr: usize, size: usize) -> usize {
        0
    }

    fn phys_to_virt(phys: usize) -> usize {
        0
    }

    fn virt_to_phys(virt: usize) -> usize {
        0
    }
}
