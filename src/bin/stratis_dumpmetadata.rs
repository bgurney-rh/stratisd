use std::env;

// use libstratis::engine::strat_engine::backstore::metadata::BDA;

fn main() {
    let args: Vec<String> = env::args().collect();
    let devpath = args[1].clone();
    println!("{}", devpath)
}
