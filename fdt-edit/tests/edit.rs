#![cfg(unix)]

use dtb_file::*;
use fdt_edit::*;
use std::fs;
use std::process::Command;

#[test]
fn test_parse_and_rebuild() {
    // Parse original DTB
    let raw_data = fdt_qemu();
    let fdt = Fdt::from_bytes(&raw_data).unwrap();
    let fdt_data = fdt.encode();

    // Create temporary files
    let temp_dir = std::env::temp_dir();
    let original_dtb_path = temp_dir.join("original.dtb");
    let rebuilt_dtb_path = temp_dir.join("rebuilt.dtb");
    let original_dts_path = temp_dir.join("original.dts");
    let rebuilt_dts_path = temp_dir.join("rebuilt.dts");

    // Cleanup function
    let cleanup = || {
        let _ = fs::remove_file(&original_dtb_path);
        let _ = fs::remove_file(&rebuilt_dtb_path);
        let _ = fs::remove_file(&original_dts_path);
        let _ = fs::remove_file(&rebuilt_dts_path);
    };

    // Ensure cleanup of temporary files
    cleanup();

    // Save original and rebuilt data to temporary files
    fs::write(&original_dtb_path, &*raw_data).expect("Failed to write original DTB file");
    fs::write(&rebuilt_dtb_path, &fdt_data).expect("Failed to write rebuilt DTB file");

    // Check if dtc command is available
    let dtc_check = Command::new("dtc").arg("--version").output();

    if dtc_check.is_err() {
        cleanup();
        panic!("dtc command not available, please install device-tree-compiler");
    }

    // Use dtc to convert DTB files to DTS files
    let original_output = Command::new("dtc")
        .args([
            "-I",
            "dtb",
            "-O",
            "dts",
            "-o",
            original_dts_path.to_str().unwrap(),
        ])
        .arg(original_dtb_path.to_str().unwrap())
        .output()
        .expect("Failed to execute dtc command (original file)");

    if !original_output.status.success() {
        cleanup();
        panic!(
            "dtc conversion of original DTB failed: {}",
            String::from_utf8_lossy(&original_output.stderr)
        );
    }

    let rebuilt_output = Command::new("dtc")
        .args([
            "-I",
            "dtb",
            "-O",
            "dts",
            "-o",
            rebuilt_dts_path.to_str().unwrap(),
        ])
        .arg(rebuilt_dtb_path.to_str().unwrap())
        .output()
        .expect("Failed to execute dtc command (rebuilt file)");

    if !rebuilt_output.status.success() {
        cleanup();
        panic!(
            "dtc conversion of rebuilt DTB failed: {}",
            String::from_utf8_lossy(&rebuilt_output.stderr)
        );
    }

    // Read generated DTS files and perform byte-by-byte comparison
    let original_dts =
        fs::read_to_string(&original_dts_path).expect("Failed to read original DTS file");
    let rebuilt_dts =
        fs::read_to_string(&rebuilt_dts_path).expect("Failed to read rebuilt DTS file");

    // Perform byte-by-byte comparison
    if original_dts != rebuilt_dts {
        println!("Original DTS file content:\n{}", original_dts);
        println!("\nRebuilt DTS file content:\n{}", rebuilt_dts);

        // Find first differing position
        let original_chars: Vec<char> = original_dts.chars().collect();
        let rebuilt_chars: Vec<char> = rebuilt_dts.chars().collect();

        let min_len = original_chars.len().min(rebuilt_chars.len());
        let mut diff_pos = None;

        for i in 0..min_len {
            if original_chars[i] != rebuilt_chars[i] {
                diff_pos = Some(i);
                break;
            }
        }

        match diff_pos {
            Some(pos) => {
                let context_start = pos.saturating_sub(50);
                let context_end = (pos + 50).min(min_len);

                println!("\nDifference found at position: {}", pos);
                println!(
                    "Original file segment: {}>>>DIFF<<<{}",
                    &original_dts[context_start..pos],
                    &original_dts[pos..context_end]
                );
                println!(
                    "Rebuilt file segment: {}>>>DIFF<<<{}",
                    &rebuilt_dts[context_start..pos],
                    &rebuilt_dts[pos..context_end]
                );
            }
            None => {
                if original_chars.len() != rebuilt_chars.len() {
                    println!(
                        "File length differs: original={}, rebuilt={}",
                        original_chars.len(),
                        rebuilt_chars.len()
                    );
                }
            }
        }

        cleanup();
        panic!("Original DTS and rebuilt DTS do not match exactly");
    }

    // Cleanup temporary files
    cleanup();

    println!("✅ Test passed: Original DTB and rebuilt DTB DTS representations match exactly");
}

// TODO: Need to implement Display trait for Fdt
// #[test]
// fn test_display_dts() {
//     // Parse DTB
//     let raw_data = fdt_qemu();
//     let fdt = Fdt::from_bytes(&raw_data).unwrap();

//     // Use Display to output DTS
//     let dts = format!("{}", fdt);

//     // Verify output format
//     assert!(dts.starts_with("/dts-v1/;"), "DTS should start with /dts-v1/;");
//     assert!(dts.contains("/ {"), "DTS should contain root node");
//     assert!(dts.contains("};"), "DTS should contain node closing");

//     // Verify it contains some common nodes
//     assert!(dts.contains("compatible"), "DTS should contain compatible property");

//     println!("✅ Display test passed");
//     println!("DTS output first 500 characters:\n{}", &dts[..dts.len().min(500)]);
// }
