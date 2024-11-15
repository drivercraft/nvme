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

    let base_vaddr = pcie_regs[0];

    info!("Init PCIE @{:?}", base_vaddr);

    let mut root = pcie::RootGeneric::new(base_vaddr.as_ptr() as usize);

    root.enumerate().for_each(|device| {
        let address = device.address();
        debug!("PCI {}", device);

        if let PciDevice::Endpoint(mut ep) = device {
            println!("{:?}", ep.id());
            let bar = ep.bar(0);
            println!("bar0: {:?}", bar);

            if ep.device_type() == DeviceType::NvmeController {
                ep.update_command(|cmd| {
                    cmd | CommandRegister::IO_ENABLE
                        | CommandRegister::MEMORY_ENABLE
                        | CommandRegister::BUS_MASTER_ENABLE
                });

                let bar_addr = match bar.unwrap() {
                    Bar::Memory32 {
                        address,
                        size,
                        prefetchable,
                    } => iomap((address as usize).into(), size as _),
                    Bar::Memory64 {
                        address,
                        size,
                        prefetchable,
                    } => iomap((address as usize).into(), size as _),
                    Bar::Io { port } => todo!(),
                };

                let nvme = Nvme::<OSImpl>::new(bar_addr).unwrap();
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
