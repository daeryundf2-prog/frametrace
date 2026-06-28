use frametrace::cli;

const CLI_STACK_SIZE: usize = 16 * 1024 * 1024;

fn main() {
    let args = std::env::args().collect();
    let result = match std::thread::Builder::new()
        .name("frametrace-cli".to_string())
        .stack_size(CLI_STACK_SIZE)
        .spawn(move || cli::run(args))
    {
        Ok(handle) => handle
            .join()
            .unwrap_or_else(|_| Err("CLI worker panicked".to_string())),
        Err(error) => Err(format!("failed to start CLI worker: {error}")),
    };

    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
