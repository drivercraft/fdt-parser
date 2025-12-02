#![cfg(not(target_os = "none"))]

#[macro_use]
extern crate log;

use dtb_file::*;
use fdt_raw::*;
use std::sync::Once;

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
fn test_phandle_display() {
    let phandle = Phandle::from(42);
    assert_eq!(format!("{}", phandle), "<0x2a>");
}

#[test]
fn test_new() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    info!("ver: {:#?}", fdt.header().version);
}

#[test]
fn test_all_nodes() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    for node in fdt.all_nodes() {
        let node = node.unwrap();
        info!("node: {}", node.name());
    }
}
