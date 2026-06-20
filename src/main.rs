fn main() {
    if let Err(err) = cargo_trustpub::app::run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}
