#[cfg(test)]
mod tests {
    use std::sync::Once;

    use dtb_file::fdt_rpi_4b;
    use fdt_edit::*;

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

        let reg = node.reg().unwrap()[0].clone();

        let parent = node.parent().unwrap();
        if let Some(addr_cells_prop) = parent.find_property("#address-cells") {
            debug!("parent #address-cells={}", addr_cells_prop.u32().unwrap());
        }
        if let Some(size_cells_prop) = parent.find_property("#size-cells") {
            debug!("parent #size-cells={}", size_cells_prop.u32().unwrap());
        }
        if let Some(ranges) = parent.ranges() {
            for (idx, range) in ranges.iter().enumerate() {
                let child_cells = range.child_bus_address().collect::<Vec<_>>();
                let parent_cells = range.parent_bus_address().collect::<Vec<_>>();
                let child_addr = child_cells
                    .iter()
                    .fold(0u64, |acc, val| (acc << 32) | (*val as u64));
                let parent_addr = parent_cells
                    .iter()
                    .fold(0u64, |acc, val| (acc << 32) | (*val as u64));
                debug!(
                    "range[{idx}]: child_cells={:?} parent_cells={:?} child={:#x} parent={:#x} size={:#x}",
                    child_cells, parent_cells, child_addr, parent_addr, range.size
                );
            }
        }

        info!("reg: {:?}", reg);

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
