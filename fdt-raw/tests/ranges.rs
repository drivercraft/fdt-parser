use std::sync::Once;

use dtb_file::fdt_rpi_4b;
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
fn test_reg() {
    let raw = fdt_rpi_4b();
    let fdt = Fdt::from_bytes(&raw).unwrap();

    let path = "/soc/serial@7e215040";

    let node = fdt.find_by_path(path).unwrap();

    let reg = node.reg().unwrap().next().unwrap();
    println!("reg: {:#x?}", reg);
    let child_bus_address = reg.address;
    let address = fdt.translate_address(path, child_bus_address);

    assert_eq!(address, 0xfe215040, "want 0xfe215040, got {:#x}", address);
    assert_eq!(
        child_bus_address, 0x7e215040,
        "want 0x7e215040, got {:#x}",
        child_bus_address
    );
    assert_eq!(
        reg.size,
        Some(0x40),
        "want 0x40, got {:#x}",
        reg.size.unwrap()
    );
}
