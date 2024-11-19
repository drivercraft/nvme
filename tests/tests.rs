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

use bare_test::fdt::PciSpace;
use bare_test::mem::dma;
use bare_test::platform::page_size;
use bare_test::time::delay;
use bare_test::{driver::device_tree::get_device_tree, mem::mmu::iomap, println};
use byte_unit::Byte;
use log::*;
use nvme_driver::*;
use pcie::preludes::*;
use pcie::PciDevice;

#[test_case]
fn test_nvme() {
    let mut nvme = get_nvme();
    let namespace_list = nvme.namespace_list().unwrap();
    for ns in &namespace_list {
        let space = Byte::from_u64(ns.lba_size as u64 * ns.lba_count as u64);

        println!("namespace: {:?}, space: {:#}", ns, space);
    }

    let ns = namespace_list[0];

    

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

fn get_nvme() -> Nvme<OSImpl> {
    let fdt = get_device_tree().unwrap();
    let pcie = fdt
        .find_compatible(&["pci-host-ecam-generic"])
        .next()
        .unwrap()
        .into_pci()
        .unwrap();

    let mut pcie_regs = alloc::vec![];

    println!("test nvme");

    println!("pcie: {}", pcie.node.name);

    for reg in pcie.node.reg().unwrap() {
        println!("pcie reg: {:#x}", reg.address);
        pcie_regs.push(iomap((reg.address as usize).into(), reg.size.unwrap()));
    }

    let mut m32_range = 0..0;
    let mut m64_range = 0..0;

    for range in pcie.ranges().unwrap() {
        match range.space {
            PciSpace::Memory32 => m32_range = range.cpu_address..range.size,
            PciSpace::Memory64 => m64_range = range.cpu_address..range.size,
            _ => {}
        }
    }

    let base_vaddr = pcie_regs[0];

    info!("Init PCIE @{:?}", base_vaddr);

    let root = pcie::RootGeneric::new(base_vaddr.as_ptr() as usize);

    for device in root.enumerate() {
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
                        bar_addr = if address == 0 {
                            let new_addr = m32_range.start as usize;
                            unsafe { ep.write_bar(slot, new_addr) };
                            new_addr
                        } else {
                            address as usize
                        };
                        bar_size = size as usize;
                    }
                    Bar::Memory64 {
                        address,
                        size,
                        prefetchable,
                    } => {
                        bar_addr = if address == 0 {
                            let new_addr = m64_range.start as usize;
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
                return nvme;
            }
        }
    }

    panic!("no nvme found");
}
