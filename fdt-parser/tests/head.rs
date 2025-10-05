#[cfg(test)]
mod test {
    use dtb_file::{fdt_phytium, fdt_rpi_4b};
    use fdt_parser::*;

    #[test]
    fn test_head() {
        let header = Header::from_bytes(&fdt_rpi_4b()).unwrap();
        println!("{:#?}", header);
    }

    #[test]
    fn test_head_phytium() {
        let raw = fdt_phytium();
        let header = Header::from_bytes(&raw).unwrap();
        println!("{:#?}", header);
    }
}
