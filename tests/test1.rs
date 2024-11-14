#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(bare_test::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate bare_test;

#[bare_test::entry]
fn main() {
    test_main();
}

use bare_test::{driver::device_tree::get_device_tree, mem::mmu::iomap, println};

#[test_case]
fn test_uart() {
    let fdt = get_device_tree().unwrap();
    unsafe {
        
        
    }

    println!("test passed!");
}
