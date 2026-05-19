use frametrace::cli;

fn main() {
    if let Err(error) = cli::run(std::env::args().collect()) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
