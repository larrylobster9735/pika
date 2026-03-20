fn main() {
    if let Err(err) = ph::run() {
        eprintln!("{err:#}");
        std::process::exit(1);
    }
}
