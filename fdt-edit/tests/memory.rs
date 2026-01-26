#![cfg(unix)]

use dtb_file::*;
use fdt_edit::NodeKind;
use fdt_edit::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_node_detection() {
        // Test memory node detection using phytium DTB
        let raw_data = fdt_phytium();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // Traverse to find memory nodes
        let mut found_memory = false;
        for node in fdt.all_nodes() {
            if let NodeKind::Memory(mem) = node.as_ref() {
                found_memory = true;
                println!("Memory node: {}", mem.name());
            }
        }
        assert!(found_memory, "Should find at least one memory node");
    }

    #[test]
    fn test_memory_regions() {
        let raw_data = fdt_phytium();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // Find memory nodes and get region information
        for node in fdt.all_nodes() {
            if let NodeKind::Memory(mem) = node.as_ref() {
                let regions = mem.regions();
                // Memory node should have at least one region
                if !regions.is_empty() {
                    for region in regions {
                        println!(
                            "Memory region: address=0x{:x}, size=0x{:x}",
                            region.address, region.size
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_memory_node_properties() {
        let raw_data = fdt_phytium();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let NodeKind::Memory(mem) = node.as_ref() {
                // Memory node should have device_type property
                let dt = mem.device_type();
                if let Some(device_type) = dt {
                    assert_eq!(device_type, "memory", "device_type should be 'memory'");
                }

                // Get node name
                let name = mem.name();
                assert!(
                    name.starts_with("memory"),
                    "Memory node name should start with 'memory'"
                );
            }
        }
    }

    #[test]
    fn test_create_memory_node() {
        // Manually create a memory node
        let mem = NodeMemory::new("memory@80000000");
        assert_eq!(mem.name(), "memory@80000000");

        // Verify initial state
        assert!(
            mem.regions().is_empty(),
            "New memory node should have no regions"
        );
    }
}
