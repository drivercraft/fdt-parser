#![cfg(unix)]

use dtb_file::*;
use fdt_edit::NodeKind;
use fdt_edit::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_node_detection() {
        // Test clock node detection using RPI 4B DTB
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // Traverse to find clock nodes (nodes with #clock-cells property)
        let mut clock_count = 0;
        for node in fdt.all_nodes() {
            if let NodeKind::Clock(clock) = node.as_ref() {
                clock_count += 1;
                println!(
                    "Clock node: {} (#clock-cells={})",
                    clock.name(),
                    clock.clock_cells
                );
            }
        }
        println!("Found {} clock nodes", clock_count);
    }

    #[test]
    fn test_clock_properties() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let NodeKind::Clock(clock) = node.as_ref() {
                // Get #clock-cells
                let cells = clock.clock_cells;
                println!("Clock: {} cells={}", clock.name(), cells);

                // Get output names
                if !clock.clock_output_names.is_empty() {
                    println!("  output-names: {:?}", clock.clock_output_names);
                }

                match &clock.kind {
                    ClockType::Fixed(fixed) => {
                        println!(
                            "  Fixed clock: freq={}Hz accuracy={:?}",
                            fixed.frequency, fixed.accuracy
                        );
                    }
                    ClockType::Normal => {
                        println!("  Clock provider");
                    }
                }
            }
        }
    }

    #[test]
    fn test_fixed_clock() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        // Find fixed clocks
        let mut found_with_freq = false;
        for node in fdt.all_nodes() {
            if let NodeKind::Clock(clock) = node.as_ref()
                && let ClockType::Fixed(fixed) = &clock.kind
            {
                // Print fixed clock information
                println!(
                    "Fixed clock found: {} freq={}Hz accuracy={:?}",
                    clock.name(),
                    fixed.frequency,
                    fixed.accuracy
                );
                // Some fixed clocks (e.g., cam1_clk, cam0_clk) don't have clock-frequency property
                if fixed.frequency > 0 {
                    found_with_freq = true;
                }
            }
        }
        // At least one fixed clock should have a frequency
        assert!(
            found_with_freq,
            "Should find at least one fixed clock with frequency"
        );
    }

    #[test]
    fn test_clock_output_name() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let NodeKind::Clock(clock) = node.as_ref() {
                let names = &clock.clock_output_names;
                if !names.is_empty() {
                    // Test output_name method
                    let first = clock.output_name(0);
                    assert_eq!(first, Some(names[0].as_str()));

                    // If there are multiple outputs, test indexed access
                    if names.len() > 1 && clock.clock_cells > 0 {
                        let second = clock.output_name(1);
                        assert_eq!(second, Some(names[1].as_str()));
                    }
                }
            }
        }
    }

    #[test]
    fn test_clock_type_conversion() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            if let NodeKind::Clock(clock) = node.as_ref() {
                match &clock.kind {
                    ClockType::Fixed(fixed) => {
                        // Print fixed clock information
                        println!(
                            "Fixed clock: {} freq={} accuracy={:?}",
                            clock.name(),
                            fixed.frequency,
                            fixed.accuracy
                        );
                    }
                    ClockType::Normal => {
                        // Test Normal type
                        println!("Clock {} is a provider", clock.name());
                    }
                }
            }
        }
    }

    #[test]
    fn test_clocks_with_context() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        let mut found_clocks = false;
        for node in fdt.all_nodes() {
            if let NodeKind::Clock(clock_ref) = node.as_ref() {
                found_clocks = true;

                let clocks = clock_ref.clocks();
                if !clocks.is_empty() {
                    found_clocks = true;
                    println!(
                        "Node: {} has {} clock references:",
                        clock_ref.name(),
                        clocks.len()
                    );
                    for (i, clk) in clocks.iter().enumerate() {
                        println!(
                            "  [{}] phandle={:?} cells={} specifier={:?} name={:?}",
                            i, clk.phandle, clk.cells, clk.specifier, clk.name
                        );
                        // Verify specifier length matches cells
                        assert_eq!(
                            clk.specifier.len(),
                            clk.cells as usize,
                            "specifier length should match cells"
                        );
                    }
                }
            }
        }
        assert!(found_clocks, "Should find nodes with clock references");
    }

    #[test]
    fn test_clock_ref_select() {
        let raw_data = fdt_rpi_4b();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            // Use as_clock_ref to get clock reference with context
            if let NodeKind::Clock(clock) = node.as_ref() {
                let clocks = clock.clocks();
                for clk in clocks {
                    // Test select() method
                    if clk.cells > 0 {
                        assert!(
                            clk.select().is_some(),
                            "select() should return Some when cells > 0"
                        );
                        assert_eq!(
                            clk.select(),
                            clk.specifier.first().copied(),
                            "select() should return first specifier"
                        );
                    }
                }
            }
        }
    }
}
