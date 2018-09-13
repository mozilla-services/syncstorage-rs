//! Application settings objects and initialization
use config::{Config, ConfigError, Environment, File};

static DEFAULT_PORT: u16 = 8000;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub debug: bool,
    pub port: u16,
    pub database_url: String,
    pub database_pool_max_size: Option<u32>,
    #[cfg(test)]
    pub database_use_test_transactions: bool,
    pub master_token_secret: Vec<u8>,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            debug: false,
            port: DEFAULT_PORT,
            database_url: "mysql://root@127.0.0.1/syncstorage".to_string(),
            database_pool_max_size: None,
            #[cfg(test)]
            database_use_test_transactions: false,
            master_token_secret: vec![],
        }
    }
}

impl Settings {
    /// Load the settings from the config file if supplied, then the environment.
    pub fn with_env_and_config_file(filename: &Option<String>) -> Result<Self, ConfigError> {
        let mut s = Config::default();
        // Set our defaults, this can be fixed up drastically later after:
        // https://github.com/mehcode/config-rs/issues/60
        s.set_default("debug", false)?;
        s.set_default("port", DEFAULT_PORT as i64)?;
        #[cfg(test)]
        s.set_default("database_use_test_transactions", false)?;
        s.set_default("master_token_secret", [0i64; 32].to_vec())?;

        // Merge the config file if supplied
        if let Some(config_filename) = filename {
            s.merge(File::with_name(config_filename))?;
        }

        // Merge the environment overrides
        s.merge(Environment::with_prefix("sync"))?;
        s.try_into()
    }
}
