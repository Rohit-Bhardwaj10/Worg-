use std::process::ExitCode;

fn main() -> ExitCode {
    match wm_platform::enumerate_windows() {
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
    }
}
