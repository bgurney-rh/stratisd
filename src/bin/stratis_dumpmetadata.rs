use std::env;

use std::fs::OpenOptions;

use libstratis::engine::strat_engine::backstore::metadata::BDA;

fn main() {
    let args: Vec<String> = env::args().collect();
    let devpath = args[1].clone();
    println!("Device path: {}", devpath);

    let bda = BDA::load(&mut OpenOptions::new().read(true).open(&devpath).unwrap()).unwrap();
    println!("{:?}", bda)
}
