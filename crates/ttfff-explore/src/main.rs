use ttfff_explore::*;

fn main() {
    let start = std::time::Instant::now();
    println!("{}", report());
    let duration = start.elapsed();
    println!("Took: {:.2?}", duration);
}