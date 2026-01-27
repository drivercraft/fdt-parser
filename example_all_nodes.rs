// Example: Using the all_nodes function
extern crate alloc;
use alloc::{string::String, vec::Vec};

use fdt_edit::{Fdt, Node, NodeRef};

fn main() {
    // Create an example FDT
    let mut fdt = Fdt::new();

    // Add some example nodes
    {
        let root = &mut fdt.root;
        let mut soc = Node::new_raw("soc");
        let mut uart = Node::new_raw("uart@4000");
        let mut gpio = Node::new_raw("gpio@5000");
        let mut led = Node::new_raw("led");

        // Set properties
        uart.add_property(fdt_edit::Property::new_str("compatible", "vendor,uart"));
        gpio.add_property(fdt_edit::Property::new_str("compatible", "vendor,gpio"));
        led.add_property(fdt_edit::Property::new_str("compatible", "vendor,led"));

        // Build the tree structure
        gpio.add_child(led);
        soc.add_child(uart);
        soc.add_child(gpio);
        root.add_child(soc);
    }

    // Use all_nodes to get all nodes
    let all_nodes: Vec<NodeRef> = fdt.all_nodes().collect();

    println!("All nodes in FDT (depth-first traversal):");
    for (i, node_ref) in all_nodes.iter().enumerate() {
        println!(
            "{}: Node '{}', Path: '{}', Depth: {}",
            i + 1,
            node_ref.node.name(),
            node_ref.context.current_path,
            node_ref.context.depth
        );

        // Display the node's compatible property
        let compatibles: Vec<&str> = node_ref.compatibles();
        if !compatibles.is_empty() {
            println!("   Compatible: {:?}", compatibles);
        }
    }

    // Use find_compatible to find specific nodes
    let uart_nodes = fdt.find_compatible(&["vendor,uart"]);
    println!("\nFound UART nodes:");
    for node_ref in uart_nodes {
        println!(
            "  Node: {}, Full path: '{}'",
            node_ref.node.name(),
            node_ref.context.current_path
        );
    }
}
