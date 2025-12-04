#[cfg(test)]
mod tests {
    use dtb_file::fdt_qemu;
    use fdt_edit::*;

    #[test]
    fn test_get_method() {
        // 解析原始 DTB
        let raw_data = fdt_qemu();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        let node = fdt.get_by_path2("/virtio_mmio@a002600");

        println!("Found node: {:#?}", node.unwrap());
    }

    #[test]
    fn test_find_method() {
        // 解析原始 DTB
        let raw_data = fdt_qemu();
        let fdt = Fdt::from_bytes(&raw_data).unwrap();

        let node = fdt.find_by_path2("/virtio_mmio");

        for n in node {
            println!("Found node {n:#?}");
        }
    }
}
