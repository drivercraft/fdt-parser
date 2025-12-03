#![no_std]

extern crate alloc;

#[cfg(test)]
mod tests {
    use alloc::{string::ToString, vec};
    use fdt_edit::{Fdt, Node, Property, RegInfo};

    #[test]
    fn test_all_nodes_empty() {
        let fdt = Fdt::new();
        let nodes = fdt.all_nodes();

        // 空的 FDT 应该只有一个根节点
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].name, ""); // 根节点名为空
    }

    #[test]
    fn test_all_nodes_simple() {
        let mut fdt = Fdt::new();

        // 添加一个子节点
        let mut child = Node::new("test-node");
        child.add_property(Property::compatible(vec!["test-compatible".to_string()]));

        fdt.root.add_child(child);

        let nodes = fdt.all_nodes();

        // 应该有根节点和一个子节点
        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].name, ""); // 根节点
        assert_eq!(nodes[1].name, "test-node"); // 子节点
    }

    #[test]
    fn test_all_nodes_multiple_children() {
        let mut fdt = Fdt::new();

        // 添加多个子节点
        let mut node1 = Node::new("node1");
        node1.add_property(Property::reg(vec![RegInfo::new(0x1000, Some(0x100))]));

        let mut node2 = Node::new("node2");
        node2.add_property(Property::compatible(vec!["test-device".to_string()]));

        let mut node3 = Node::new("node3");

        fdt.root.add_child(node1);
        fdt.root.add_child(node2);
        fdt.root.add_child(node3);

        let nodes = fdt.all_nodes();

        // 应该有根节点和三个子节点
        assert_eq!(nodes.len(), 4);
        assert_eq!(nodes[0].name, ""); // 根节点
        assert_eq!(nodes[1].name, "node1");
        assert_eq!(nodes[2].name, "node2");
        assert_eq!(nodes[3].name, "node3");
    }

    #[test]
    fn test_all_nodes_nested() {
        let mut fdt = Fdt::new();

        // 创建嵌套结构：root -> soc -> uart -> clock
        let mut uart = Node::new("uart");
        uart.add_property(Property::reg(vec![RegInfo::new(0x9000000, Some(0x1000))]));

        let mut clock = Node::new("clock");
        clock.add_property(Property::compatible(vec!["fixed-clock".to_string()]));

        uart.add_child(clock);

        let mut soc = Node::new("soc");
        soc.add_property(Property::compatible(vec!["simple-bus".to_string()]));
        soc.add_child(uart);

        fdt.root.add_child(soc);

        let nodes = fdt.all_nodes();

        // 应该有根节点 -> soc -> uart -> clock，共4个节点
        assert_eq!(nodes.len(), 4);
        assert_eq!(nodes[0].name, "");      // 根节点
        assert_eq!(nodes[1].name, "soc");   // 第一级子节点
        assert_eq!(nodes[2].name, "uart");  // 第二级子节点
        assert_eq!(nodes[3].name, "clock"); // 第三级子节点
    }

    #[test]
    fn test_all_nodes_complex_hierarchy() {
        let mut fdt = Fdt::new();

        // 构建复杂层次结构：
        // root
        // ├── soc
        // │   ├── uart0
        // │   └── uart1
        // └── memory

        let mut uart0 = Node::new("uart0");
        uart0.add_property(Property::reg(vec![RegInfo::new(0x9000000, Some(0x1000))]));

        let mut uart1 = Node::new("uart1");
        uart1.add_property(Property::reg(vec![RegInfo::new(0x9001000, Some(0x1000))]));

        let mut soc = Node::new("soc");
        soc.add_property(Property::compatible(vec!["simple-bus".to_string()]));
        soc.add_child(uart0);
        soc.add_child(uart1);

        let mut memory = Node::new("memory");
        memory.add_property(Property::reg(vec![RegInfo::new(0x40000000, Some(0x10000000))]));

        fdt.root.add_child(soc);
        fdt.root.add_child(memory);

        let nodes = fdt.all_nodes();

        // 应该有根节点 + soc + uart0 + uart1 + memory，共5个节点
        // 深度优先遍历：根节点 -> soc -> uart0 -> uart1 -> memory
        assert_eq!(nodes.len(), 5);
        assert_eq!(nodes[0].name, "");       // 根节点
        assert_eq!(nodes[1].name, "soc");    // 第一级子节点1（按插入顺序，深度优先遍历）
        assert_eq!(nodes[2].name, "uart0");  // soc的子节点1
        assert_eq!(nodes[3].name, "uart1");  // soc的子节点2
        assert_eq!(nodes[4].name, "memory"); // 第一级子节点2（按插入顺序，在soc的子树处理完后）
    }

    #[test]
    fn test_all_nodes_depth_first_order() {
        let mut fdt = Fdt::new();

        // 构建测试结构以验证深度优先遍历顺序
        let mut child1 = Node::new("child1");
        let mut grandchild1 = Node::new("grandchild1");
        child1.add_child(grandchild1);

        let mut child2 = Node::new("child2");
        let mut grandchild2 = Node::new("grandchild2");
        child2.add_child(grandchild2);

        fdt.root.add_child(child1);
        fdt.root.add_child(child2);

        let nodes = fdt.all_nodes();

        // 验证深度优先遍历顺序
        assert_eq!(nodes.len(), 5);
        assert_eq!(nodes[0].name, "");           // 根节点
        assert_eq!(nodes[1].name, "child1");     // 第一个子节点
        assert_eq!(nodes[2].name, "grandchild1"); // child1的子节点
        assert_eq!(nodes[3].name, "child2");     // 第二个子节点
        assert_eq!(nodes[4].name, "grandchild2"); // child2的子节点
    }

    #[test]
    fn test_all_nodes_with_properties() {
        let mut fdt = Fdt::new();

        // 根节点添加属性
        fdt.root.add_property(Property::AddressCells(1));
        fdt.root.add_property(Property::SizeCells(1));

        // 子节点添加属性
        let mut test_node = Node::new("test");
        test_node.add_property(Property::compatible(vec!["test-compatible".to_string()]));
        test_node.add_property(Property::reg(vec![RegInfo::new(0x2000, Some(0x1000))]));

        fdt.root.add_child(test_node);

        let nodes = fdt.all_nodes();

        // 验证节点数量
        assert_eq!(nodes.len(), 2);

        // 验证根节点属性
        assert_eq!(nodes[0].properties.len(), 2); // address-cells, size-cells

        // 验证子节点属性
        assert_eq!(nodes[1].properties.len(), 2); // compatible, reg
    }
}