// 示例：使用 all_nodes 函数
extern crate alloc;
use alloc::{string::String, vec::Vec};

use fdt_edit::{Fdt, Node, NodeRef};

fn main() {
    // 创建一个示例 FDT
    let mut fdt = Fdt::new();

    // 添加一些示例节点
    {
        let root = &mut fdt.root;
        let mut soc = Node::new_raw("soc");
        let mut uart = Node::new_raw("uart@4000");
        let mut gpio = Node::new_raw("gpio@5000");
        let mut led = Node::new_raw("led");

        // 设置属性
        uart.add_property(fdt_edit::Property::new_str("compatible", "vendor,uart"));
        gpio.add_property(fdt_edit::Property::new_str("compatible", "vendor,gpio"));
        led.add_property(fdt_edit::Property::new_str("compatible", "vendor,led"));

        // 构建树结构
        gpio.add_child(led);
        soc.add_child(uart);
        soc.add_child(gpio);
        root.add_child(soc);
    }

    // 使用 all_nodes 获取所有节点
    let all_nodes: Vec<NodeRef> = fdt.all_nodes().collect();

    println!("FDT 中所有节点 (深度优先遍历):");
    for (i, node_ref) in all_nodes.iter().enumerate() {
        println!(
            "{}: 节点 '{}', 路径: '{}', 深度: {}",
            i + 1,
            node_ref.node.name(),
            node_ref.context.current_path,
            node_ref.context.depth
        );

        // 显示节点的 compatible 属性
        let compatibles: Vec<&str> = node_ref.compatibles();
        if !compatibles.is_empty() {
            println!("   Compatible: {:?}", compatibles);
        }
    }

    // 使用 find_compatible 查找特定节点
    let uart_nodes = fdt.find_compatible(&["vendor,uart"]);
    println!("\n找到 UART 节点:");
    for node_ref in uart_nodes {
        println!(
            "  节点: {}, 完整路径: '{}'",
            node_ref.node.name(),
            node_ref.context.current_path
        );
    }
}
