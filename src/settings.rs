//! Application settings objects and initialization

use config::{Config, ConfigError, Environment, File};
use serde::de::{Deserialize, Deserializer};

use web::auth::hkdf_expand_32;

static DEFAULT_PORT: u16 = 8000;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub debug: bool,
    pub port: u16,
    pub database_url: String,
    pub database_pool_max_size: Option<u32>,
    #[cfg(test)]
    pub database_use_test_transactions: bool,
    pub master_secret: Secrets,
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
            master_secret: Secrets::default(),
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
        s.set_default("master_secret", "")?;

        // Merge the config file if supplied
        if let Some(config_filename) = filename {
            s.merge(File::with_name(config_filename))?;
        }

        // Merge the environment overrides
        s.merge(Environment::with_prefix("sync"))?;
        s.try_into()
    }
}

#[derive(Debug)]
pub struct Secrets {
    pub master_secret: Vec<u8>,
    pub signing_secret: [u8; 32],
}

impl Secrets {
    pub fn new(master_secret: &str) -> Secrets {
        let master_secret = master_secret.as_bytes().to_vec();
        let signing_secret = hkdf_expand_32(
            b"services.mozilla.com/tokenlib/v1/signing",
            None,
            &master_secret,
        );
        Secrets {
            master_secret,
            signing_secret,
        }
    }
}

impl Default for Secrets {
    fn default() -> Secrets {
        Secrets {
            master_secret: vec![],
            signing_secret: [0u8; 32],
        }
    }
}

impl<'d> Deserialize<'d> for Secrets {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'d>,
    {
        let master_secret: String = Deserialize::deserialize(deserializer)?;
        Ok(Secrets::new(&master_secret))
    }
}
