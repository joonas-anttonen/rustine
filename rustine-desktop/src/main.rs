use std::env;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        eprintln!("Usage: {}", args[0]);
        return std::process::ExitCode::from(1);
    }

    println!("rustine-desktop");
    
    std::process::ExitCode::SUCCESS
}
