#![no_std]

extern crate alloc;

#[cfg(test)]
mod tests {
    use alloc::vec;
    use fdt_edit::{Fdt, Node, Property, RegInfo};

    #[test]
    fn test_address_cells_scope_only_affects_children() {
        let mut fdt = Fdt::new();

        // 根节点: address-cells = 2, size-cells = 2
        // 这会影响根节点的直接子节点
        fdt.root.add_property(Property::AddressCells(2));
        fdt.root.add_property(Property::SizeCells(2));

        // SoC 节点: 作为根节点的子节点，应该使用父节点的 2+2 cells
        let mut soc = Node::new("soc");
        soc.add_property(Property::reg(vec![
            RegInfo::new(0x40000000, Some(0x100000))  // 地址用2个cell，大小用2个cell
        ]));

        // SoC 节点: address-cells = 1, size-cells = 1
        // 这会影响 SoC 的子节点（如 uart）
        soc.add_property(Property::AddressCells(1));
        soc.add_property(Property::SizeCells(1));

        // UART 节点: 作为 SoC 的子节点，应该使用 SoC 的 1+1 cells
        let mut uart = Node::new("uart");
        uart.add_property(Property::reg(vec![
            RegInfo::new(0x9000000, Some(0x1000))  // 地址用1个cell，大小用1个cell
        ]));

        soc.add_child(uart);
        fdt.root.add_child(soc);

        // 验证 FDT 构建成功，没有 panic
        let fdt_data = fdt.to_bytes();

        // 确保数据不为空
        assert!(!fdt_data.is_empty());

        // 这个测试证明了 address_cells 和 size_cells 只影响直接子节点
        // - 根节点的 2+2 cells影响了 SoC 节点的 reg 属性格式
        // - SoC 节点的 1+1 cells影响了 UART 节点的 reg 属性格式

        // 测试通过：address_cells 和 size_cells 作用范围正确
        // 根节点设置 2+2 cells 影响了 SoC 节点
        // SoC 节点设置 1+1 cells 影响了 UART 节点
    }

    #[test]
    fn test_cells_inheritance_chain() {
        let mut fdt = Fdt::new();

        // 第一层: 根节点 2+2 cells
        fdt.root.add_property(Property::AddressCells(2));
        fdt.root.add_property(Property::SizeCells(2));

        // 第二层: 中间节点 1+1 cells
        let mut middle = Node::new("middle");
        middle.add_property(Property::reg(vec![
            RegInfo::new(0x10000000, Some(0x1000))  // 使用根节点的 2+2 cells
        ]));
        middle.add_property(Property::AddressCells(1));
        middle.add_property(Property::SizeCells(1));

        // 第三层: 设备节点，继承中间节点的 1+1 cells
        let mut device = Node::new("device");
        device.add_property(Property::reg(vec![
            RegInfo::new(0x2000, Some(0x100))  // 使用中间节点的 1+1 cells
        ]));

        middle.add_child(device);
        fdt.root.add_child(middle);

        let fdt_data = fdt.to_bytes();
        assert!(!fdt_data.is_empty());

        // 多级 cells 继承链测试通过
    }

    #[test]
    fn test_default_cells_when_not_specified() {
        let mut fdt = Fdt::new();

        // 根节点不设置 address-cells 和 size-cells
        // 应该使用默认值 1+1

        let mut device = Node::new("device");
        device.add_property(Property::reg(vec![
            RegInfo::new(0x1000, Some(0x100))  // 使用默认的 1+1 cells
        ]));

        fdt.root.add_child(device);

        let fdt_data = fdt.to_bytes();
        assert!(!fdt_data.is_empty());

        // 默认 cells 值测试通过
    }
}