use frametrace::{cli, serve};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() <= 1 {
        // Running the bare executable (e.g. double-click) launches the local
        // examiner workstation instead of printing CLI usage.
        if let Err(error) = serve::run(serve::ServeOptions {
            case_dir: None,
            port: None,
        }) {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
        return;
    }
    if let Err(error) = cli::run(args) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
