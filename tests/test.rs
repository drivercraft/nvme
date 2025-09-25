#![no_std]
#![no_main]
#![feature(used_with_arg)]

extern crate alloc;
extern crate bare_test;

#[bare_test::tests]
mod tests {

    use core::ffi::CStr;

    use alloc::{ffi::CString, format};
    use bare_test::{
        fdt_parser::PciSpace,
        globals::{global_val, PlatformInfoKind},
        mem::iomap,
        platform::page_size,
        println,
    };
    use byte_unit::Byte;
    use log::*;
    use nvme_driver::*;
    use pcie::{
        enumerate_by_controller, CommandRegister, DeviceType, PciMem32, PciMem64, PcieController,
        PcieGeneric,
    };

    #[test]
    fn test_nvme() {
        let mut nvme = get_nvme();
        let namespace_list = nvme
            .namespace_list()
            .inspect_err(|e| error!("{e:?}"))
            .unwrap();
        for ns in &namespace_list {
            let space = Byte::from_u64(ns.lba_size as u64 * ns.lba_count as u64);

            println!("namespace: {:?}, space: {:#}", ns, space);
        }

        for _i in 0..128 {
            let _ = nvme
                .namespace_list()
                .inspect_err(|e| error!("{e:?}"))
                .unwrap();
        }

        println!("admin queue test ok");

        let ns = namespace_list[0];

        for i in 0..128 {
            let want_str = format!("hello world! block {i}");

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
        let PlatformInfoKind::DeviceTree(fdt) = &global_val().platform_info;
        let fdt = fdt.get();
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
        let base_vaddr = pcie_regs[0];

        let mut drv = PcieController::new(PcieGeneric::new(base_vaddr));

        for range in pcie.ranges().unwrap() {
            info!("{range:?}");
            match range.space {
                PciSpace::Memory32 => {
                    drv.set_mem32(
                        PciMem32 {
                            address: range.cpu_address as _,
                            size: range.size as _,
                        },
                        range.prefetchable,
                    );
                }
                PciSpace::Memory64 => {
                    drv.set_mem64(
                        PciMem64 {
                            address: range.cpu_address as _,
                            size: range.size as _,
                        },
                        range.prefetchable,
                    );
                }
                _ => {}
            }
        }

        let base_vaddr = pcie_regs[0];

        info!("Init PCIE @{base_vaddr:?}");

        let page_size = page_size();

        for mut ep in enumerate_by_controller(&mut drv, None) {
            println!("{}", ep);
            println!("  BARs:");
            for i in 0..6 {
                if let Some(bar) = ep.bar(i) {
                    println!("    BAR{}: {:x?}", i, bar);
                }
            }
            for cap in ep.capabilities() {
                println!("  {:?}", cap);
            }
            if ep.device_type() == DeviceType::NvmeController {
                let bar = ep.bar_mmio(0).unwrap();

                println!("bar0: [{:#x}, {:#x})", bar.start, bar.end);

                let addr = iomap(bar.start.into(), bar.count());

                ep.update_command(|mut cmd| {
                    cmd.insert(CommandRegister::BUS_MASTER_ENABLE | CommandRegister::MEMORY_ENABLE);
                    cmd
                });

                let nvme = Nvme::new(
                    addr,
                    Config {
                        page_size,
                        io_queue_pair_count: 1,
                    },
                )
                .inspect_err(|e| error!("{e:?}"))
                .unwrap();

                return nvme;
            }
        }

        panic!("no nvme found");
    }
}
