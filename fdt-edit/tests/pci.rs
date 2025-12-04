#[cfg(test)]
mod tests {
    use dtb_file::fdt_qemu;
    use fdt_edit::*;

    #[test]
    fn test_pci_node_detection() {
        let raw_data = fdt_qemu();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // Try to find PCI nodes
        let mut pci_nodes_found = 0;
        for node in fdt.all_nodes() {
            {
                if let Node::Pci(pci) = node {
                    pci_nodes_found += 1;
                    println!("Found PCI node: {}", pci.name());
                }
            }
        }

        // We should find at least one PCI node in the qemu PCI test file
        assert!(pci_nodes_found > 0, "Should find at least one PCI node");
    }

    #[test]
    fn test_bus_range() {
        let raw_data = fdt_qemu();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            {
                if let Node::Pci(pci) = node
                    && let Some(range) = pci.bus_range()
                {
                    println!("Found bus-range: {range:?}");
                    assert!(range.start <= range.end, "Bus range start should be <= end");
                    return; // Test passed
                }
            }
        }

        // println!("No bus-range found in any PCI node");
    }

    #[test]
    fn test_pci_properties() {
        let raw_data = fdt_qemu();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            {
                if let Node::Pci(pci) = node {
                    // Test address cells
                    assert_eq!(
                        pci.address_cells(),
                        Some(3),
                        "PCI should use 3 address cells"
                    );

                    // Test interrupt cells
                    assert_eq!(pci.interrupt_cells(), 1, "PCI should use 1 interrupt cell");

                    // Test device type
                    if let Some(device_type) = pci.device_type() {
                        assert!(!device_type.is_empty());
                    }

                    // Test compatibles
                    let compatibles = pci.compatibles();
                    if !compatibles.is_empty() {
                        println!("Compatibles: {:?}", compatibles);
                    }

                    return; // Test passed for first PCI node found
                }
            }
        }

        panic!("No PCI nodes found for property testing");
    }
}
