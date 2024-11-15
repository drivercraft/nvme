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

use core::alloc::Layout;
use core::sync::atomic::fence;

use bare_test::mem::dma;
use bare_test::platform::page_size;
use bare_test::time::delay;
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

    let mut pcie_ranges = alloc::vec![];

    for range in pcie.ranges() {
        println!("pcie range: {:?}", range);

        pcie_ranges.push(range);
    }

    let base_vaddr = pcie_regs[0];

    info!("Init PCIE @{:?}", base_vaddr);

    let root = pcie::RootGeneric::new(base_vaddr.as_ptr() as usize);

    let bar64_range = pcie_ranges[2];

    root.enumerate().for_each(|device| {
        debug!("PCI {}", device);

        if let PciDevice::Endpoint(mut ep) = device {
            println!("{:?}", ep.id());
            ep.update_command(|cmd| {
                cmd | CommandRegister::IO_ENABLE
                    | CommandRegister::MEMORY_ENABLE
                    | CommandRegister::BUS_MASTER_ENABLE
            });

            if ep.device_type() == DeviceType::NvmeController {
                let mut addr = None;
                let slot = 0;
                let bar = ep.bar(slot).unwrap();

                println!("bar{}: {:?}", slot, bar);

                let bar_addr;
                let bar_size;

                match bar {
                    Bar::Memory32 {
                        address,
                        size,
                        prefetchable,
                    } => {
                        bar_addr = address as usize;
                        bar_size = size as usize;
                    }
                    Bar::Memory64 {
                        address,
                        size,
                        prefetchable,
                    } => {
                        bar_addr = if address == 0 {
                            let new_addr = bar64_range.parent_bus_address as usize;
                            unsafe { ep.write_bar(slot, new_addr) };
                            new_addr
                        } else {
                            address as usize
                        };
                        bar_size = size as usize;
                    }
                    Bar::Io { port } => todo!(),
                };

                if slot == 0 {
                    addr = Some(iomap(bar_addr.into(), bar_size));
                }

                let nvme = Nvme::<OSImpl>::new(addr.unwrap()).unwrap();
            }
        }
    });

    println!("test passed!");
}

pub struct OSImpl;

impl OS for OSImpl {
    fn sleep(duration: core::time::Duration) {
        todo!()
    }

    fn dma_alloc(layout: core::alloc::Layout) -> Option<DMAMem> {
        unsafe {
            dma::alloc_coherent(layout).map(|m| DMAMem {
                virt: m.cpu_addr,
                phys: m.bus_addr.as_u64(),
                layout,
            })
        }
    }

    fn dma_dealloc(dma: DMAMem) {
        unsafe {
            dma::dealloc_coherent(
                dma::DMAMem {
                    cpu_addr: dma.virt,
                    bus_addr: dma.phys.into(),
                },
                dma.layout,
            );
        }
    }

    fn page_size() -> usize {
        unsafe { page_size() }
    }
}
