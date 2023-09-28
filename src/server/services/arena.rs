use std::fs;
use std::io;
use std::path::Path;

static DEFAULT_CONFIG_CONTENT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/cgarena_config.toml"
));

const CONFIG_FILE_NAME: &str = "cgarena_config.toml";
const BOTS_DIR_NAME: &str = "bots";

pub fn create_new_arena(path: &Path) -> Result<(), io::Error> {
    match fs::create_dir(path) {
        Ok(_) => (),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => (),
        e => return e,
    }

    let config_file_path = path.join(CONFIG_FILE_NAME);
    std::fs::write(config_file_path, DEFAULT_CONFIG_CONTENT)?;

    let bots_dir_path = path.join(BOTS_DIR_NAME);
    fs::create_dir(bots_dir_path)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use tempdir::TempDir;

    #[test]
    fn new_arena_can_be_created() {
        let tmp_dir = TempDir::new("cgarena").unwrap();
        let path = tmp_dir.path().join("test");
        let res = create_new_arena(&path);
        assert!(res.is_ok(), "Arena creation failed {:?}", res.err());
        assert!(path.join("cgarena_config.toml").exists());
        assert!(path.join("bots").is_dir());
    }
}
