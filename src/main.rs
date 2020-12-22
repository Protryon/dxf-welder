use std::env;

#[macro_use]
mod result;
pub use result::*;

mod dxf_process;
use dxf_process::*;

mod dxf;

fn main() {
    let args = env::args().into_iter().skip(1).collect::<Vec<String>>();
    let infile = args.get(0).expect("no input file");
    let outfile = args.get(1).expect("no output file");
    let config = DxfConfig {
        resolution: 0.05,
        max_radius: 100000.0,
        min_segments: 3,
    };
    let input = std::fs::read_to_string(infile).expect("failed to read dxf");
    let parsed = dxf::Drawing::parse(&input).expect("failed to parse dxf");
    let out_drawing = config.process_drawing(parsed).expect("failed to process dxf file");
    std::fs::write(&outfile, out_drawing.to_string()).expect("failed to write dxf file");
}
