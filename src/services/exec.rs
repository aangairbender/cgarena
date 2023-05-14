use std::process::{Command, Stdio};

use crate::models::Language;

const SOURCE_FILE_TEMPLATE: &str = "{SOURCE_FILE}";

pub fn build_source_code(
    bot_name: &str,
    filename: &str,
    language: &Language,
) -> Result<(), String> {
    if let Some(build_cmd) = language.build_cmd.as_ref() {
        let tokens = build_cmd
            .iter()
            .map(|t| t.replace(SOURCE_FILE_TEMPLATE, filename))
            .collect::<Vec<String>>();
        let build_output = Command::new(&tokens[0])
            .args(tokens.iter().skip(1))
            .current_dir(std::env::current_dir().unwrap().join("bots").join(bot_name))
            .output()
            .expect("Build command should be executed");
        if build_output.status.success() {
            Ok(())
        } else {
            Err(std::str::from_utf8(&build_output.stderr)
                .expect("Build errors should be in utf-8")
                .to_owned())
        }
    } else {
        Ok(())
    }
}

pub fn health_check(language: &Language) -> bool {
    let tokens = &language.health_check_cmd;

    Command::new(&tokens[0])
        .args(tokens.iter().skip(1))
        .stdout(Stdio::piped())
        .status()
        .expect("Health check command should be executed")
        .success()
}
