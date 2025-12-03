#![no_std]

extern crate alloc;

#[cfg(test)]
mod tests {
    use alloc::{string::String, string::ToString, vec};
    use core::fmt::Write;

    use fdt_edit::{Fdt, Node, Property, RegEntry};

    #[test]
    fn test_display_simple_fdt() {
        let mut fdt = Fdt::new();

        // 设置基本属性
        fdt.root.add_property(Property::AddressCells(1));
        fdt.root.add_property(Property::SizeCells(1));
        fdt.root.add_property(Property::Compatible(vec!["test,soc".to_string()]));

        let mut cpu_node = Node::new("cpu@0");
        cpu_node.add_property(Property::DeviceType("cpu".to_string()));
        cpu_node.add_property(Property::Compatible(vec!["arm,cortex-a53".to_string()]));
        cpu_node.add_property(Property::Reg {
            entries: vec![RegEntry::new(0x0, Some(0x1000))],
            address_cells: 1,
            size_cells: 1,
        });
        cpu_node.add_property(Property::Status(fdt_raw::Status::Okay));

        fdt.root.add_child(cpu_node);

        // 测试格式化
        let mut output = String::new();
        write!(&mut output, "{}", fdt).unwrap();

        // 验证基本结构
        assert!(output.contains("// Device Tree Source"));
        assert!(output.contains("/ {"));
        assert!(output.contains("#address-cells = <1>"));
        assert!(output.contains("#size-cells = <1>"));
        assert!(output.contains("compatible = \"test,soc\""));
        assert!(output.contains("cpu@0 {"));
        assert!(output.contains("device_type = \"cpu\""));
        assert!(output.contains("compatible = \"arm,cortex-a53\""));
        assert!(output.contains("reg = <0x0 0x1000>"));
        assert!(output.contains("status = \"okay\""));
        assert!(output.contains("};"));
    }

    #[test]
    fn test_display_with_memory_reservation() {
        let mut fdt = Fdt::new();
        fdt.memory_reservations.push(fdt_edit::MemoryReservation {
            address: 0x80000000,
            size: 0x100000,
        });

        fdt.root.add_property(Property::Model("Test Board".to_string()));

        let mut output = String::new();
        write!(&mut output, "{}", fdt).unwrap();

        assert!(output.contains("/dts-v1/;"));
        assert!(output.contains("/memreserve/"));
        assert!(output.contains("0x80000000 0x100000;"));
        assert!(output.contains("model = \"Test Board\""));
    }

    #[test]
    fn test_display_nested_structure() {
        let mut fdt = Fdt::new();

        fdt.root.add_property(Property::AddressCells(1));
        fdt.root.add_property(Property::SizeCells(1));

        let mut bus_node = Node::new("bus@1000");
        bus_node.add_property(Property::Reg {
            entries: vec![RegEntry::new(0x1000, Some(0x100))],
            address_cells: 1,
            size_cells: 1,
        });

        let mut device1_node = Node::new("device1@2000");
        device1_node.add_property(Property::Reg {
            entries: vec![RegEntry::new(0x2000, Some(0x50))],
            address_cells: 1,
            size_cells: 1,
        });
        device1_node.add_property(Property::Status(fdt_raw::Status::Okay));

        let mut device2_node = Node::new("device2@3000");
        device2_node.add_property(Property::Reg {
            entries: vec![RegEntry::new(0x3000, Some(0x50))],
            address_cells: 1,
            size_cells: 1,
        });
        device2_node.add_property(Property::Status(fdt_raw::Status::Disabled));

        bus_node.add_child(device1_node);
        bus_node.add_child(device2_node);
        fdt.root.add_child(bus_node);

        let mut output = String::new();
        write!(&mut output, "{}", fdt).unwrap();

        assert!(output.contains("bus@1000 {"));
        assert!(output.contains("reg = <0x1000 0x100>"));
        assert!(output.contains("device1@2000 {"));
        assert!(output.contains("reg = <0x2000 0x50>"));
        assert!(output.contains("status = \"okay\""));
        assert!(output.contains("device2@3000 {"));
        assert!(output.contains("reg = <0x3000 0x50>"));
        assert!(output.contains("status = \"disabled\""));
    }
}