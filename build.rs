use std::process::Command;

fn main() {
    let _ = Command::new("deno")
        .arg("run")
        .arg("build")
        .current_dir("frontend")
        .status()
        .map_err(|_| {
            Command::new("npm")
                .arg("run")
                .arg("build")
                .current_dir("frontend")
                .status()
        });
}
