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

    #[test]
    fn test_set_regs_with_ranges_conversion() {
        init_logging();
        let raw = fdt_rpi_4b();
        let mut fdt = Fdt::from_bytes(&raw).unwrap();

        // 获取可变节点引用
        let mut node = fdt.get_by_path_mut("/soc/serial@7e215040").unwrap();

        // 获取原始 reg 信息
        let original_regs = node.regs().unwrap();
        let original_reg = original_regs[0];
        info!("Original reg: {:#x?}", original_reg);

        // Set regs using CPU address (0xfe215040 is CPU address)
        // set_regs should convert it to bus address (0x7e215040) when storing
        let new_cpu_address = 0xfe215080u64; // New CPU address
        let new_size = 0x80u64;
        node.set_regs(&[RegInfo {
            address: new_cpu_address,
            size: Some(new_size),
        }]);

        // Re-read to verify
        let updated_regs = node.regs().unwrap();
        let updated_reg = updated_regs[0];
        info!("Updated reg: {:#x?}", updated_reg);

        // Verify: CPU address read back should be what we set
        assert_eq!(
            updated_reg.address, new_cpu_address,
            "CPU address should be {:#x}, got {:#x}",
            new_cpu_address, updated_reg.address
        );

        // Verify: bus address should be the converted value
        // 0xfe215080 - 0xfe000000 + 0x7e000000 = 0x7e215080
        let expected_bus_address = 0x7e215080u64;
        assert_eq!(
            updated_reg.child_bus_address, expected_bus_address,
            "Bus address should be {:#x}, got {:#x}",
            expected_bus_address, updated_reg.child_bus_address
        );

        assert_eq!(
            updated_reg.size,
            Some(new_size),
            "Size should be {:#x}, got {:?}",
            new_size,
            updated_reg.size
        );
    }

    #[test]
    fn test_set_regs_roundtrip() {
        init_logging();
        let raw = fdt_rpi_4b();
        let mut fdt = Fdt::from_bytes(&raw).unwrap();

        // Get original reg information
        let original_reg = {
            let node = fdt.get_by_path("/soc/serial@7e215040").unwrap();
            node.regs().unwrap()[0]
        };
        info!("Original reg: {:#x?}", original_reg);

        // Set regs again using the same CPU address
        {
            let mut node = fdt.get_by_path_mut("/soc/serial@7e215040").unwrap();
            node.set_regs(&[RegInfo {
                address: original_reg.address, // Use CPU address
                size: original_reg.size,
            }]);
        }

        // Verify roundtrip: reading back should be same as original
        let roundtrip_reg = {
            let node = fdt.get_by_path("/soc/serial@7e215040").unwrap();
            node.regs().unwrap()[0]
        };
        info!("Roundtrip reg: {:#x?}", roundtrip_reg);

        assert_eq!(
            roundtrip_reg.address, original_reg.address,
            "Roundtrip CPU address mismatch"
        );
        assert_eq!(
            roundtrip_reg.child_bus_address, original_reg.child_bus_address,
            "Roundtrip bus address mismatch"
        );
        assert_eq!(
            roundtrip_reg.size, original_reg.size,
            "Roundtrip size mismatch"
        );
    }
}
