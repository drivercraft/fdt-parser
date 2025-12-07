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

        // 使用 CPU 地址设置 reg (0xfe215040 是 CPU 地址)
        // set_regs 应该将其转换为 bus 地址 (0x7e215040) 后存储
        let new_cpu_address = 0xfe215080u64; // 新的 CPU 地址
        let new_size = 0x80u64;
        node.set_regs(&[RegInfo {
            address: new_cpu_address,
            size: Some(new_size),
        }]);

        // 重新读取验证
        let updated_regs = node.regs().unwrap();
        let updated_reg = updated_regs[0];
        info!("Updated reg: {:#x?}", updated_reg);

        // 验证：读取回来的 CPU 地址应该是我们设置的值
        assert_eq!(
            updated_reg.address, new_cpu_address,
            "CPU address should be {:#x}, got {:#x}",
            new_cpu_address, updated_reg.address
        );

        // 验证：bus 地址应该是转换后的值
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

        // 获取原始 reg 信息
        let original_reg = {
            let node = fdt.get_by_path("/soc/serial@7e215040").unwrap();
            node.regs().unwrap()[0]
        };
        info!("Original reg: {:#x?}", original_reg);

        // 使用相同的 CPU 地址重新设置 reg
        {
            let mut node = fdt.get_by_path_mut("/soc/serial@7e215040").unwrap();
            node.set_regs(&[RegInfo {
                address: original_reg.address, // 使用 CPU 地址
                size: original_reg.size,
            }]);
        }

        // 验证 roundtrip：读取回来应该和原来一样
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
