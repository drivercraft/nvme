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

use core::{alloc::Layout, ffi::CStr};

use alloc::{ffi::CString, format, vec::Vec};
use bare_test::{
    driver::device_tree::get_device_tree,
    fdt::PciSpace,
    mem::{dma, mmu::iomap},
    platform::page_size,
    println,
};
use byte_unit::Byte;
use log::*;
use nvme_driver::*;
use pcie::{CommandRegister, DeviceType, Header, RootComplexGeneric, SimpleBarAllocator};

#[test_case]
fn test_nvme() {
    let mut nvme = get_nvme();
    let namespace_list = nvme
        .namespace_list()
        .inspect_err(|e| error!("{:?}", e))
        .unwrap();
    for ns in &namespace_list {
        let space = Byte::from_u64(ns.lba_size as u64 * ns.lba_count as u64);

        println!("namespace: {:?}, space: {:#}", ns, space);
    }

    for _i in 0..128 {
        let _ = nvme
            .namespace_list()
            .inspect_err(|e| error!("{:?}", e))
            .unwrap();
    }

    println!("admin queue test ok");

    let ns = namespace_list[0];

    for i in 0..128 {
        let want_str = format!("hello world! block {}", i);

        let want = CString::new(want_str.as_str()).unwrap();

        let want_bytes = want.to_bytes();

        // buff 大小需与块大小一致
        let mut write_buff = alloc::vec![0u8; ns.lba_size];

        write_buff[0..want_bytes.len()].copy_from_slice(want_bytes);

        nvme.block_write_sync(&ns, i, &write_buff).unwrap();

        let mut buff = alloc::vec![0u8; ns.lba_size];

        nvme.block_read_sync(&ns, i, &mut buff).unwrap();

        let read_result = unsafe { CStr::from_ptr(buff.as_ptr() as _) }.to_str();

        println!("read result: {:?}", read_result.unwrap());

        assert_eq!(Ok(want_str.as_str()), read_result);
    }

    println!("test passed!");
}

fn get_nvme() -> Nvme {
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

    let mut bar_alloc = SimpleBarAllocator::default();

    for range in pcie.ranges().unwrap() {
        info!("pcie range: {:?}", range);

        match range.space {
            PciSpace::Memory32 => bar_alloc.set_mem32(range.cpu_address as _, range.size as _),
            PciSpace::Memory64 => bar_alloc.set_mem64(range.cpu_address, range.size),
            _ => {}
        }
        // match range.space {
        //     PciSpace::Memory32 => bar_alloc.set_mem64((range.cpu_address + 0x1000) as _, range.size as _),
        //     // PciSpace::Memory64 => bar_alloc.set_mem64(range.cpu_address, range.size),
        //     _ => {}
        // }
    }

    let base_vaddr = pcie_regs[0];

    info!("Init PCIE @{:?}", base_vaddr);

    let page_size = unsafe { page_size() };

    let mut root = RootComplexGeneric::new(base_vaddr);

    for elem in root.enumerate(None, Some(bar_alloc)) {
        debug!("PCI {}", elem);

        if let Header::Endpoint(ep) = elem.header {
            ep.update_command(elem.root, |cmd| {
                cmd | CommandRegister::IO_ENABLE
                    | CommandRegister::MEMORY_ENABLE
                    | CommandRegister::BUS_MASTER_ENABLE
            });

            if ep.device_type() == DeviceType::NvmeController {
                let bar_addr;
                let bar_size;
                match ep.bar {
                    pcie::BarVec::Memory32(bar_vec_t) => {
                        let bar0 = bar_vec_t[0].as_ref().unwrap();
                        bar_addr = bar0.address as usize;
                        bar_size = bar0.size as usize;
                    }
                    pcie::BarVec::Memory64(bar_vec_t) => {
                        let bar0 = bar_vec_t[0].as_ref().unwrap();
                        bar_addr = bar0.address as usize;
                        bar_size = bar0.size as usize;
                    }
                    pcie::BarVec::Io(_bar_vec_t) => todo!(),
                };

                println!("bar0: {:#x}", bar_addr);

                let addr = iomap(bar_addr.into(), bar_size);

                let nvme = Nvme::new(
                    addr,
                    Config {
                        page_size,
                        io_queue_pair_count: 1,
                    },
                )
                .inspect_err(|e| error!("{:?}", e))
                .unwrap();
                return nvme;
            }
        }
    }

    panic!("no nvme found");
}
