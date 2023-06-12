use std::{
    fs::{self, File},
    io::Write,
};

use crate::{
    config::{Config, CONFIG_FILE},
    models::Language,
};

pub fn create_new_arena(name: &str) {
    let path = std::env::current_dir().unwrap().join(name);

    fs::create_dir(&path).unwrap();

    let mut config = Config::default();

    let languages = vec![Language {
        name: "cpp".to_string(),
        file_extension: "cpp".to_string(),
        health_check_cmd: vec!["g++".to_string(), "--version".to_string()],
        build_cmd: Some(vec![
            "g++".to_string(),
            "--std=c++17".to_string(),
            "-Og".to_string(),
            "-o bot".to_string(),
            "{SOURCE_FILE}".to_string(),
        ]),
        run_cmd: vec!["./bot".to_string()],
    }];
    config.languages = languages;

    let toml_content = toml::to_string(&config).expect("Default config should be serializable");

    let filename = path.join(CONFIG_FILE);
    let mut file = File::create(filename).unwrap();

    file.write_all(toml_content.as_bytes()).unwrap();

    let bots_folder = path.join("bots");
    fs::create_dir(bots_folder).unwrap();
}
