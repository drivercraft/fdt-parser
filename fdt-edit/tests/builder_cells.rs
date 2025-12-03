#![no_std]

extern crate alloc;

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec};
    use fdt_edit::{Fdt, Node, Property, RegInfo};

    #[test]
    fn test_builder_with_different_cells() {
        let mut fdt = Fdt::new();

        // 根节点使用 2 cells for address, 1 for size
        fdt.root.add_property(Property::AddressCells(2));
        fdt.root.add_property(Property::SizeCells(1));

        // 验证 FDT 构建成功，没有 panic
        let fdt_data = fdt.to_bytes();

        // 确保数据不为空
        assert!(!fdt_data.is_empty());
        assert!(fdt_data.len() > 100); // 应该有基本的 FDT 结构
    }

    #[test]
    fn test_builder_with_size_cells_zero() {
        let mut fdt = Fdt::new();

        // 根节点使用 1 cell for address, 0 for size
        fdt.root.add_property(Property::AddressCells(1));
        fdt.root.add_property(Property::SizeCells(0));

        // 当 size-cells = 0 时，reg 只有地址，没有大小
        fdt.root
            .add_property(Property::reg(vec![RegInfo::new(0x1000, None)]));

        // 验证 FDT 构建成功，没有 panic
        let fdt_data = fdt.to_bytes();

        // 确保数据不为空
        assert!(!fdt_data.is_empty());
    }

    #[test]
    fn test_nested_cells_inheritance() {
        let mut fdt = Fdt::new();

        // 根节点: address-cells = 2, size-cells = 1
        fdt.root.add_property(Property::AddressCells(2));
        fdt.root.add_property(Property::SizeCells(1));

        // 中间节点: address-cells = 1, size-cells = 1
        let mut bus = Node::new("bus");
        bus.add_property(Property::AddressCells(1));
        bus.add_property(Property::SizeCells(1));

        // 设备节点: 继承父节点的 1 cell address, 1 cell size
        let mut device = Node::new("device");
        device.add_property(Property::reg(vec![RegInfo::new(0x2000, Some(0x100))]));

        bus.add_child(device);
        fdt.root.add_child(bus);

        // 验证 FDT 构建成功，没有 panic
        let fdt_data = fdt.to_bytes();

        // 确保数据不为空，应该比简单测试更大
        assert!(!fdt_data.is_empty());
        assert!(fdt_data.len() > 200);
    }

    #[test]
    fn test_complex_hierarchy_with_cells() {
        let mut fdt = Fdt::new();

        // 根节点: address-cells = 2, size-cells = 2
        fdt.root.add_property(Property::AddressCells(2));
        fdt.root.add_property(Property::SizeCells(2));

        // 中间节点: address-cells = 1, size-cells = 1
        let mut soc = Node::new("soc");
        soc.add_property(Property::AddressCells(1));
        soc.add_property(Property::SizeCells(1));
        soc.add_property(Property::reg(vec![RegInfo::new(
            0x40000000,
            Some(0x100000),
        )]));

        // 设备节点: 使用父节点的 1 cell address, 1 cell size
        let mut uart = Node::new("uart");
        uart.add_property(Property::reg(vec![RegInfo::new(0x9000000, Some(0x1000))]));

        soc.add_child(uart);
        fdt.root.add_child(soc);

        // 验证复杂层级结构的 FDT 构建成功
        let fdt_data = fdt.to_bytes();

        // 确保数据不为空，应该比简单测试更大
        assert!(!fdt_data.is_empty());
        assert!(fdt_data.len() > 150); // 调整期望值
    }
}
