use fdt_parser::Fdt;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Trace)
        .init();

    let bytes = include_bytes!("../../dtb-file/src/dtb/phytium.dtb");

    let fdt = Fdt::from_bytes(bytes).unwrap();

    // Find memory nodes by compatible string
    let memory_nodes = fdt.find_compatible(&["memory"]);

    for memory_node in memory_nodes {
        println!("Memory node: {}", memory_node.name());

        // Note: regions() method may not be available in cache parser
        // for region in memory_node.regions() {
        //     println!(" {:?}", region);
        // }

        // Print some basic info about the memory node
        let compatibles = memory_node.compatibles();
        if !compatibles.is_empty() {
            println!("  Compatibles: {:?}", compatibles);
        }
    }
}
