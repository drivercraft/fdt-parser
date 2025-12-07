#[cfg(test)]
mod tests {
    use std::sync::Once;

    use dtb_file::fdt_rpi_4b;
    use fdt_edit::*;
    use log::*;

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
    fn test_reg() {
        init_logging();
        let raw = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw).unwrap();

        let node = fdt.get_by_path("/soc/serial@7e215040").unwrap();

        let reg = node.regs().unwrap()[0];

        info!("reg: {:#x?}", reg);

        assert_eq!(
            reg.address, 0xfe215040,
            "want 0xfe215040, got {:#x}",
            reg.address
        );

        assert_eq!(
            reg.child_bus_address, 0x7e215040,
            "want 0x7e215040, got {:#x}",
            reg.child_bus_address
        );
        assert_eq!(
            reg.size,
            Some(0x40),
            "want 0x40, got {:#x}",
            reg.size.unwrap()
        );
    }
}
