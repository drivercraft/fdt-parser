use fdt_parser::Fdt;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    let bytes = include_bytes!("../../dtb/phytium.dtb");

    let fdt = Fdt::from_bytes(bytes).unwrap();
    for memory in fdt.memory() {
        println!("{}", memory.name());
        for region in memory.regions() {
            println!(" {:?}", region);
        }
    }
}
