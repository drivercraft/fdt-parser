#![cfg(unix)]

use dtb_file::*;
use fdt_edit::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fdt_display() {
        // Test Display functionality using RPI 4B DTB
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // Test Display output
        let dts_output = format!("{}", fdt);

        // Verify output contains DTS header
        assert!(dts_output.contains("/dts-v1/;"));

        // Verify output contains root node
        assert!(dts_output.contains("/ {"));

        println!("FDT Display output:\n{}", dts_output);
    }

    #[test]
    fn test_fdt_debug() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // Test Debug output
        let debug_output = format!("{:?}", fdt);

        // Verify Debug output contains struct information
        assert!(debug_output.contains("Fdt"));
        assert!(debug_output.contains("boot_cpuid_phys"));

        println!("FDT Debug output:\n{}", debug_output);
    }

    #[test]
    fn test_node_display() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // Find a node to test
        for node in fdt.all_nodes() {
            if node.name().contains("gpio") {
                let dts_output = format!("{}", node);

                // Verify output contains node name
                assert!(dts_output.contains("gpio"));

                println!("Node Display output:\n{}", dts_output);
                break;
            }
        }
    }

    #[test]
    fn test_node_debug() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if node.name().contains("gpio") {
                let debug_output = format!("{:?}", node);

                // Verify Debug output contains Node struct information
                assert!(debug_output.contains("NodeRef"));
                assert!(debug_output.contains("name"));

                println!("Node Debug output:\n{}", debug_output);
                break;
            }
        }
    }

    #[test]
    fn test_clock_node_display() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let NodeKind::Clock(clock) = node.as_ref() {
                let display_output = format!("{}", clock);
                let debug_output = format!("{:?}", clock);

                println!("Clock Node Display:\n{}", display_output);
                println!("Clock Node Debug:\n{}", debug_output);

                // Verify output contains clock-related information
                assert!(display_output.contains("Clock Node"));

                // Verify Debug contains detailed information
                assert!(debug_output.contains("NodeRefClock"));
                assert!(debug_output.contains("clock_cells"));

                break;
            }
        }
    }

    #[test]
    fn test_interrupt_controller_display() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let NodeKind::InterruptController(ic) = node.as_ref() {
                let display_output = format!("{}", ic);
                let debug_output = format!("{:?}", ic);

                println!("Interrupt Controller Display:\n{}", display_output);
                println!("Interrupt Controller Debug:\n{}", debug_output);

                // Verify output contains interrupt controller-related information
                assert!(display_output.contains("Interrupt Controller"));

                // Verify Debug contains detailed information
                assert!(debug_output.contains("NodeRefInterruptController"));
                assert!(debug_output.contains("interrupt_cells"));

                break;
            }
        }
    }

    #[test]
    fn test_memory_node_display() {
        let raw_data = fdt_phytium();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let NodeKind::Memory(mem) = node.as_ref() {
                let display_output = format!("{}", mem);
                let debug_output = format!("{:?}", mem);

                println!("Memory Node Display:\n{}", display_output);
                println!("Memory Node Debug:\n{}", debug_output);

                // Verify output contains memory-related information
                assert!(display_output.contains("Memory Node"));

                // Verify Debug contains detailed information
                assert!(debug_output.contains("NodeRefMemory"));
                assert!(debug_output.contains("regions_count"));

                break;
            }
        }
    }

    #[test]
    fn test_noderef_display_with_details() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if node.name().contains("clock") {
                let display_output = format!("{}", node);

                println!("NodeRef Display with details:\n{}", display_output);

                // Verify output contains type information
                assert!(display_output.contains("Clock Node"));

                break;
            }
        }
    }

    #[test]
    fn test_create_simple_fdt() {
        let fdt = Fdt::new();

        // Test basic Display functionality
        let dts_output = format!("{}", fdt);
        println!("Created FDT Display:\n{}", dts_output);

        // Verify output contains basic header
        assert!(dts_output.contains("/dts-v1/;"));
        assert!(dts_output.contains("/ {"));
    }

    #[test]
    fn test_manual_node_display() {
        let node = Node::new("test-node");

        // Test basic Display functionality
        let display_output = format!("{}", node);
        println!("Manual Node Display:\n{}", display_output);

        // Verify output contains node name
        assert!(display_output.contains("test-node"));

        // Test Debug
        let debug_output = format!("{:?}", node);
        println!("Manual Node Debug:\n{}", debug_output);

        assert!(debug_output.contains("Node"));
        assert!(debug_output.contains("test-node"));
    }

    #[test]
    fn test_fdt_deep_debug() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // Test basic Debug format
        let simple_debug = format!("{:?}", fdt);
        println!("FDT Simple Debug:\n{}", simple_debug);

        // Verify basic format contains basic information
        assert!(simple_debug.contains("Fdt"));
        assert!(simple_debug.contains("boot_cpuid_phys"));

        // Test deep Debug format
        let deep_debug = format!("{:#?}", fdt);
        println!("FDT Deep Debug:\n{}", deep_debug);

        // Verify deep format contains node information
        assert!(deep_debug.contains("Fdt {"));
        assert!(deep_debug.contains("nodes:"));
        assert!(deep_debug.contains("[000]"));

        // Verify it contains specific node types
        assert!(
            deep_debug.contains("Clock")
                || deep_debug.contains("InterruptController")
                || deep_debug.contains("Memory")
                || deep_debug.contains("Generic")
        );
    }

    #[test]
    fn test_fdt_deep_debug_with_simple_tree() {
        let mut fdt = Fdt::new();

        // Create a simple tree structure for testing
        let mut soc = Node::new("soc");
        soc.set_property(Property::new("#address-cells", vec![0x1, 0x0, 0x0, 0x0]));
        soc.set_property(Property::new("#size-cells", vec![0x1, 0x0, 0x0, 0x0]));

        let mut uart = Node::new("uart@9000000");
        uart.set_property(Property::new("compatible", b"arm,pl011\0".to_vec()));
        uart.set_property(Property::new(
            "reg",
            vec![
                0x00, 0x90, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00,
            ],
        ));
        uart.set_property(Property::new("status", b"okay\0".to_vec()));

        soc.add_child(uart);
        fdt.root.add_child(soc);

        // Test deep debug output
        let deep_debug = format!("{:#?}", fdt);
        println!("Simple Tree Deep Debug:\n{}", deep_debug);

        // Verify output contains expected node information
        assert!(deep_debug.contains("[000] : Generic"));
        assert!(deep_debug.contains("[001] soc: Generic"));
        assert!(deep_debug.contains("[002] uart@9000000: Generic"));
        assert!(deep_debug.contains("#address-cells=1"));
        assert!(deep_debug.contains("#size-cells=1"));
    }
}
