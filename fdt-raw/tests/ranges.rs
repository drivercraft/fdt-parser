use std::sync::Once;

use dtb_file::{fdt_qemu, fdt_rpi_4b};
use fdt_raw::Fdt;

fn init_logging() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Trace)
            .try_init();
    });
}

#[test]
fn test_ranges() {
    init_logging();

    let raw = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    let node = fdt.find_by_path("/soc").unwrap();

    let ranges = node.ranges().unwrap();
    for range in ranges.iter() {
        println!("{range:#x?}");
    }
}

#[test]
fn test_memory() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();
    let memory = fdt.memory().unwrap();
    println!("Memory node: {:#x?}", memory);
}
