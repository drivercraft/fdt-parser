use dtb_file::*;
use fdt_edit::*;

#[test]
fn test_all_node() {
    // Test memory node detection using phytium DTB
    let raw_data = fdt_phytium();
    let fdt = Fdt::from_bytes(&raw_data).unwrap();
    for node in fdt.all_raw_nodes() {
        println!("Node: {:?}", node);
    }
}
