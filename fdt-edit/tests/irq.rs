#![cfg(unix)]

use dtb_file::*;
use fdt_edit::NodeKind;
use fdt_edit::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_controller_detection() {
        // Test interrupt controller node detection using RPI 4B DTB
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // Traverse to find interrupt controller nodes
        let mut irq_count = 0;
        for node in fdt.all_nodes() {
            if let NodeKind::InterruptController(ic) = node.as_ref() {
                irq_count += 1;
                println!(
                    "Interrupt controller: {} (#interrupt-cells={:?})",
                    ic.name(),
                    ic.interrupt_cells()
                );
            }
        }
        println!("Found {} interrupt controllers", irq_count);
        assert!(
            irq_count > 0,
            "Should find at least one interrupt controller"
        );
    }

    #[test]
    fn test_interrupt_controller_properties() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let NodeKind::InterruptController(ic) = node.as_ref() {
                // Get #interrupt-cells
                let cells = ic.interrupt_cells();
                println!("IRQ Controller: {} cells={:?}", ic.name(), cells);

                // Get #address-cells (if present)
                let addr_cells = ic.interrupt_address_cells();
                if addr_cells.is_some() {
                    println!("  #address-cells: {:?}", addr_cells);
                }

                // Verify is_interrupt_controller
                assert!(
                    ic.is_interrupt_controller(),
                    "Should be marked as interrupt controller"
                );

                // Get compatible list
                let compat = ic.compatibles();
                if !compat.is_empty() {
                    println!("  compatible: {:?}", compat);
                }
            }
        }
    }

    #[test]
    fn test_interrupt_controller_by_name() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // Find GIC (ARM Generic Interrupt Controller)
        let mut found_gic = false;
        for node in fdt.all_nodes() {
            if let NodeKind::InterruptController(ic) = node.as_ref() {
                let compat = ic.compatibles();
                if compat.iter().any(|c| c.contains("gic")) {
                    found_gic = true;
                    println!("Found GIC: {}", ic.name());

                    // GIC typically has 3 interrupt-cells
                    let cells = ic.interrupt_cells();
                    println!("  #interrupt-cells: {:?}", cells);
                }
            }
        }
        // Note: Not all DTBs have GIC, this is just an example
        if found_gic {
            println!("GIC found in this DTB");
        }
    }

    #[test]
    fn test_interrupt_controller_with_phytium() {
        // Phytium DTB should have interrupt controllers
        let raw_data = fdt_phytium();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        let mut controllers = Vec::new();
        for node in fdt.all_nodes() {
            if let NodeKind::InterruptController(ic) = node.as_ref() {
                controllers.push((
                    ic.name().to_string(),
                    ic.interrupt_cells(),
                    ic.compatibles().join(", "),
                ));
            }
        }

        println!("Interrupt controllers in Phytium DTB:");
        for (name, cells, compat) in &controllers {
            println!(
                "  {} (#interrupt-cells={:?}, compatible={})",
                name, cells, compat
            );
        }

        assert!(
            !controllers.is_empty(),
            "Phytium should have at least one interrupt controller"
        );
    }

    #[test]
    fn test_interrupt_controller_detection_logic() {
        // Test whether nodes are correctly identified as interrupt controllers
        let raw_data = fdt_qemu();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            let name = node.name();
            let is_ic = matches!(node.as_ref(), NodeKind::InterruptController(_));

            // If node name starts with interrupt-controller, it should be detected
            if name.starts_with("interrupt-controller") && !is_ic {
                println!(
                    "Warning: {} might be an interrupt controller but not detected",
                    name
                );
            }

            // If node has interrupt-controller property, it should be detected
            if node.find_property("interrupt-controller").is_some() && !is_ic {
                println!(
                    "Warning: {} has interrupt-controller property but not detected",
                    name
                );
            }
        }
    }

    #[test]
    fn test_interrupt_cells_values() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let NodeKind::InterruptController(ic) = node.as_ref()
                && let Some(cells) = ic.interrupt_cells()
            {
                // Common interrupt-cells values: 1, 2, 3
                assert!(
                    (1..=4).contains(&cells),
                    "Unusual #interrupt-cells value: {} for {}",
                    cells,
                    ic.name()
                );
            }
        }
    }
}
