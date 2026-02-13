//! Clock node view tests.

use dtb_file::*;
use fdt_edit::{Fdt, NodeType};

#[test]
fn test_clock_node_detection() {
    let raw_data = fdt_phytium();
    let fdt = Fdt::from_bytes(&raw_data).unwrap();

    let mut clock_count = 0;
    for node in fdt.all_nodes() {
        if let NodeType::Clock(clock) = node {
            clock_count += 1;
            println!(
                "Clock node: {} #clock-cells={}",
                clock.path(),
                clock.clock_cells()
            );
        }
    }

    println!("Total clock nodes: {}", clock_count);
    // 飞腾 DTB 应该有时钟节点
    assert!(clock_count > 0, "phytium DTB should have clock nodes");
}

#[test]
fn test_clock_output_names() {
    let raw_data = fdt_phytium();
    let fdt = Fdt::from_bytes(&raw_data).unwrap();

    for node in fdt.all_nodes() {
        if let NodeType::Clock(clock) = node {
            let names = clock.clock_output_names();
            if !names.is_empty() {
                println!(
                    "Clock {} has output names: {:?}",
                    clock.path(),
                    names
                );

                // Test output_name method
                if let Some(first_name) = clock.output_name(0) {
                    assert_eq!(first_name, names[0]);
                    println!("  First output: {}", first_name);
                }
            }
        }
    }
}

#[test]
fn test_fixed_clock() {
    let raw_data = fdt_phytium();
    let fdt = Fdt::from_bytes(&raw_data).unwrap();

    for node in fdt.all_nodes() {
        if let NodeType::Clock(clock) = node {
            let clock_type = clock.clock_type();
            if let fdt_edit::ClockType::Fixed(fixed) = clock_type {
                println!(
                    "Fixed clock: {} freq={}Hz",
                    clock.path(),
                    fixed.frequency
                );

                // Fixed clock should have a frequency
                assert!(
                    fixed.frequency > 0 || fixed.accuracy.is_some(),
                    "Fixed clock should have frequency or accuracy"
                );

                if let Some(ref name) = fixed.name {
                    println!("  Name: {}", name);
                }

                if let Some(accuracy) = fixed.accuracy {
                    println!("  Accuracy: {} ppb", accuracy);
                }
            }
        }
    }
}
