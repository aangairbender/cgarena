use std::path::Path;

static DEFAULT_CONFIG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/assets/cgarena_config.toml"));

pub struct ArenaService {}

impl ArenaService {
    pub fn init<P: AsRef<Path>>(path: P) {

    }
}

#[cfg(test)]
mod test {
    use crate::config::Config;

    use super::*;

    #[test]
    fn default_config_can_be_deserialized() {
        let config = toml::from_str::<Config>(DEFAULT_CONFIG);
        assert!(config.is_ok());
    }
}