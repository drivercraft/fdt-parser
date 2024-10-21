use clap::Parser;
use fdt_parser::Fdt;
use std::io::Write;

/// Simple DTB parser
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// dtb file path
    #[arg(short, long)]
    input: String,

    /// output file path
    #[arg(short, long)]
    output: String,
}

fn main() {
    let args = Args::parse();

    let data = std::fs::read(&args.input).unwrap();

    let fdt = Fdt::from_bytes(&data).unwrap();

    let _ = std::fs::remove_file(&args.output);
    let mut file = std::fs::File::create(&args.output).unwrap();

    writeln!(file, "/dts-v{}/;", fdt.version()).unwrap();
    for region in fdt.memory_reservation_block() {
        writeln!(file, "/memreserve/ {:?};", region).unwrap();
    }

    for node in fdt.all_nodes() {
        let space = "\t".repeat(node.level - 1);
        writeln!(file, "{}{}", space, node.name()).unwrap();

        if let Some(cap) = node.compatible() {
            writeln!(file, "{} -compatible: ", space).unwrap();
            for cap in cap {
                writeln!(file, "{}     {:?}", space, cap.unwrap()).unwrap();
            }
        }

        if let Some(reg) = node.reg() {
            writeln!(file, "{} - reg: ", space).unwrap();
            for cell in reg {
                writeln!(file, "{}     {:?}", space, cell).unwrap();
            }
        }
    }
}
