use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match (args.next().as_deref(), args.next()) {
        (Some("dump"), None) => match wm_platform::enumerate_windows() {
            Ok(snapshot) => match serde_json::to_string_pretty(&snapshot) {
                Ok(json) => {
                    println!("{json}");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("failed to encode snapshot: {error}");
                    ExitCode::FAILURE
                }
            },
            Err(error) => {
                eprintln!("failed to enumerate windows: {error}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("usage: wmctl dump");
            ExitCode::from(2)
        }
    }
}
