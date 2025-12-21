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
fn test_rsv1() {
    init_logging();

    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    for node in fdt.reserved_memory() {
        println!("reserved memory node: {}", node.name());
        let ranges = node.ranges().unwrap();
        for range in ranges.iter() {
            println!("  range: {range:#x?}");
        }
    }
}

#[test]
fn test_rsv2() {
    init_logging();

    let raw = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    for node in fdt.reserved_memory() {
        println!("reserved memory node: {}", node.name());
    }

    let want_names = ["linux,cma", "nvram@0", "nvram@1"];

    for (i, node) in fdt.reserved_memory().enumerate() {
        assert_eq!(node.name(), want_names[i]);
    }
}
