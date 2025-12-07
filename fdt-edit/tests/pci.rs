#[cfg(test)]
mod tests {
    use dtb_file::{fdt_phytium, fdt_qemu};
    use fdt_edit::*;

    #[test]
    fn test_pci_node_detection() {
        let raw_data = fdt_qemu();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // Try to find PCI nodes
        let mut pci_nodes_found = 0;
        for node in fdt.all_nodes() {
            {
                if let NodeRef::Pci(pci) = node {
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
                if let NodeRef::Pci(pci) = node
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
                if let NodeRef::Pci(pci) = node {
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
                    let compatibles = pci.compatibles().collect::<Vec<_>>();
                    if !compatibles.is_empty() {
                        println!("Compatibles: {:?}", compatibles);
                    }

                    return; // Test passed for first PCI node found
                }
            }
        }

        panic!("No PCI nodes found for property testing");
    }

    #[test]
    fn test_pci2() {
        let raw = fdt_phytium();
        let fdt = Fdt::from_bytes(&raw).unwrap();
        let node = fdt
            .find_compatible(&["pci-host-ecam-generic"])
            .into_iter()
            .next()
            .unwrap();

        let NodeRef::Pci(pci) = node else {
            panic!("Not a PCI node");
        };

        let want = [
            PciRange {
                space: PciSpace::IO,
                bus_address: 0x0,
                cpu_address: 0x50000000,
                size: 0xf00000,
                prefetchable: false,
            },
            PciRange {
                space: PciSpace::Memory32,
                bus_address: 0x58000000,
                cpu_address: 0x58000000,
                size: 0x28000000,
                prefetchable: false,
            },
            PciRange {
                space: PciSpace::Memory64,
                bus_address: 0x1000000000,
                cpu_address: 0x1000000000,
                size: 0x1000000000,
                prefetchable: false,
            },
        ];

        for (i, range) in pci.ranges().unwrap().iter().enumerate() {
            assert_eq!(*range, want[i]);
            println!("{range:#x?}");
        }
    }

    #[test]
    fn test_pci_irq_map() {
        let raw = fdt_phytium();
        let fdt = Fdt::from_bytes(&raw).unwrap();
        let node_ref = fdt
            .find_compatible(&["pci-host-ecam-generic"])
            .into_iter()
            .next()
            .unwrap();

        let NodeRef::Pci(pci) = node_ref else {
            panic!("Not a PCI node");
        };

        let irq = pci.child_interrupts(0, 0, 0, 4).unwrap();

        assert!(!irq.irqs.is_empty());
    }

    #[test]
    fn test_pci_irq_map2() {
        let raw = fdt_qemu();
        let fdt = Fdt::from_bytes(&raw).unwrap();
        let node_ref = fdt
            .find_compatible(&["pci-host-ecam-generic"])
            .into_iter()
            .next()
            .unwrap();

        let NodeRef::Pci(pci) = node_ref else {
            panic!("Not a PCI node");
        };

        let irq = pci.child_interrupts(0, 2, 0, 1).unwrap();

        let want = [0, 5, 4];

        for (got, want) in irq.irqs.iter().zip(want.iter()) {
            assert_eq!(*got, *want);
        }
    }
}
