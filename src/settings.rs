//! Application settings objects and initialization

use std::env::var;

use config::{Config, ConfigError, Environment, File};
use serde::de::{Deserialize, Deserializer};

use web::auth::hkdf_expand_32;

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
        Settings::with_env_and_config_file(None).unwrap()
    }
}

impl Settings {
    /// Construct a `Settings` instance, populating it with data from the file
    /// system and local environment.
    ///
    /// Precedence (earlier items override later ones):
    ///
    ///   1. Environment variables: `$SYNC_<UPPERCASE_KEY_NAME>`
    ///   2. File: `filename` argument
    ///   3. File: `config/local.toml`
    ///   4. File: `config/<$SYNC_ENV>.toml`
    ///   5. File: `config/default.toml`
    pub fn with_env_and_config_file(filename: Option<String>) -> Result<Self, ConfigError> {
        let mut config = Config::new();

        config.merge(File::with_name("config/default"))?;

        let env = var("SYNC_ENV").unwrap_or_else(|_| "dev".to_string());
        config.merge(File::with_name(&format!("config/{}", env)).required(false))?;
        config.set_default("env", "dev")?;

        config.merge(File::with_name("config/local").required(false))?;

        if let Some(config_filename) = filename {
            config.merge(File::with_name(&config_filename))?;
        }

        config.merge(Environment::with_prefix("sync"))?;

        config.try_into().and_then(|settings: Settings| {
            if env == "production" && settings.master_secret.master_secret.len() == 0 {
                Err(ConfigError::NotFound("master_secret".to_string()))
            } else {
                Ok(settings)
            }
        })
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
