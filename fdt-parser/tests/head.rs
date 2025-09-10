#[cfg(test)]
mod test {
    use dtb_file::{fdt_phytium, fdt_rpi_4b};
    use fdt_parser::*;

    #[test]
    fn test_head() {
        let raw = fdt_rpi_4b();
        let ptr = raw.as_ptr() as *mut u8;
        let header = Header::from_ptr(ptr).unwrap();
        println!("{:#?}", header);
    }

    #[test]
    fn test_head_phytium() {
        let raw = fdt_phytium();
        let ptr = raw.as_ptr() as *mut u8;
        let header = Header::from_ptr(ptr).unwrap();
        println!("{:#?}", header);
    }
}
