#[cfg(test)]
mod tests {
    use dtb_file::fdt_qemu;
    use fdt_edit::*;

    #[test]
    fn test_get_method() {
        // Parse the original DTB
        let raw_data = fdt_qemu();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        let node = fdt.get_by_path("/virtio_mmio@a002600");

        println!("Found node: {:#?}", node.unwrap());
    }

    #[test]
    fn test_find_method() {
        // Parse the original DTB
        let raw_data = fdt_qemu();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        let node = fdt.find_by_path("/virtio_mmio");

        for n in node {
            println!("Found node {n:#?}");
        }

        let count = fdt.find_by_path("/virtio_mmio").count();
        println!("Total found nodes: {}", count);
        assert_eq!(count, 32);
    }

    #[test]
    fn test_all() {
        // Parse the original DTB
        let raw_data = fdt_qemu();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes() {
            println!("Node: {:#?}", node);
            println!("  {}", node.path());
            println!("-------------------------");
        }

        let count = fdt.all_nodes().count();
        println!("Total nodes: {}", count);
        assert_eq!(count, 56);
    }

    #[test]
    fn test_all_mut() {
        // Parse the original DTB
        let raw_data = fdt_qemu();
        let mut fdt = Fdt::from_bytes(&raw_data).unwrap();

        for node in fdt.all_nodes_mut() {
            println!("Node: {:#?}", node);
            println!("  {}", node.path());
            println!("-------------------------");
        }

        let count = fdt.all_nodes().count();
        println!("Total nodes: {}", count);
        assert_eq!(count, 56);
    }
}
