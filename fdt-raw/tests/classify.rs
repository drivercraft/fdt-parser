use std::sync::Once;

use dtb_file::fdt_qemu;
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
fn test_chosen() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();
    let chosen = fdt.chosen().unwrap();
    println!("Chosen node: {:?}", chosen);
}

#[test]
fn test_memory() {
    init_logging();
    let raw = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw).unwrap();
    let memory = fdt.memory().next().unwrap();
    println!("Memory node: {:#x?}", memory);
}
