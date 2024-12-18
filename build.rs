use std::process::Command;

const WEB_UI_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/cg-arena-ui");

fn main() {
    npm()
        .arg("install")
        .current_dir(WEB_UI_PATH)
        .status()
        .unwrap();

    npm()
        .arg("run")
        .arg("build")
        .current_dir(WEB_UI_PATH)
        .status()
        .unwrap();
}

fn npm() -> Command {
    #[cfg(windows)]
    const NPM: &str = "npm.cmd";
    #[cfg(not(windows))]
    const NPM: &str = "npm";

    Command::new(NPM)
}
