#[cfg(test)]
mod tests;

use clap::Parser;
use config::builder::DefaultState;
use config::ConfigBuilder;
use serde::{Deserialize, Serialize};
use settings_loader::common::database::DatabaseSettings;
use settings_loader::common::http::HttpServerSettings;
use settings_loader::{Environment, LoadingOptions, SettingsError, SettingsLoader};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Settings {
    pub application: HttpServerSettings,
    pub database: DatabaseSettings,

    #[serde(flatten)]
    pub correlation: CorrelationSettings,
}

impl SettingsLoader for Settings {
    type Options = CliOptions;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorrelationSettings {
    /// Specify the machine id [0, 31) used in correlation id generation, overriding what may be set
    /// in an environment variable. This id should be unique for the entity type within a cluster
    /// environment. Different entity types can use the same machine id.
    #[serde(deserialize_with = "deser_string_or_i32")]
    pub machine_id: i32,

    /// Specify the node id [0, 31) used in correlation id generation, overriding what may be set
    /// in an environment variable. This id should be unique for the entity type within a cluster
    /// environment. Different entity types can use the same machine id.
    #[serde(deserialize_with = "deser_string_or_i32")]
    pub node_id: i32,
}

impl Default for CorrelationSettings {
    fn default() -> Self {
        Self { machine_id: 1, node_id: 1 }
    }
}

#[derive(Debug, Default, Parser, PartialEq)]
#[clap(author, version, about)]
pub struct CliOptions {
    /// Explicit configuration to load, bypassing inferred configuration load mechanism. If this
    /// option is used, the application + environment will be ignored; however, secrets, env var,
    /// and explicit overrides will still be used.
    ///
    /// Default behavior is to infer-load configuration based on `APP_ENVIRONMENT` envvar.
    #[clap(short, long, value_name = "PATH_TO_CONFIG_FILE")]
    pub config: Option<PathBuf>,

    /// specify path to secrets configuration file
    #[clap(long, value_name = "PATH_TO_SECRETS_FILE")]
    pub secrets: Option<PathBuf>,

    /// specify the environment configuration override used in inferred configuration load.
    #[clap(short = 'e', long = "env")]
    pub environment: Option<Environment>,

    /// Override filesystem path used to search for application and environment configuration files.
    /// Directories are separated by the ':' character.
    /// Default path is "./resources".
    #[clap(short = 's', long = "search-path", value_name = "SETTINGS_SEARCH_PATH")]
    pub settings_search_path: Option<String>,

    /// Specify the machine id [0, 31) used in correlation id generation, overriding what may be set
    /// in an environment variable. This id should be unique for the entity type within a cluster
    /// environment. Different entity types can use the same machine id.
    /// Optionally overrides the engine.machine_id setting.
    #[clap(short, long, value_name = "[0, 31)")]
    pub machine_id: Option<i8>,

    /// Specify the node id [0, 31) used in correlation id generation, overriding what may be set
    /// in an environment variable. This id should be unique for the entity type within a cluster
    /// environment. Different entity types can use the same machine id.
    /// Optionally override the engine.node_id setting.
    #[clap(short, long, value_name = "[0, 31)")]
    pub node_id: Option<i8>,
}

const DEFAULT_SEARCH_PATH: &str = "./resources";

impl LoadingOptions for CliOptions {
    type Error = SettingsError;

    fn config_path(&self) -> Option<PathBuf> {
        self.config.clone()
    }

    fn secrets_path(&self) -> Option<PathBuf> {
        self.secrets.clone()
    }

    fn implicit_search_paths(&self) -> Vec<PathBuf> {
        let search_path = self.settings_search_path.as_deref().unwrap_or(DEFAULT_SEARCH_PATH);
        search_path.split(':').map(PathBuf::from).collect()
    }

    fn load_overrides(
        &self, config: ConfigBuilder<DefaultState>,
    ) -> Result<ConfigBuilder<DefaultState>, Self::Error> {
        let config = match self.machine_id {
            None => config,
            Some(machine_id) => config.set_override("machine_id", i64::from(machine_id))?,
        };

        let config = match self.node_id {
            None => config,
            Some(node_id) => config.set_override("node_id", i64::from(node_id))?,
        };

        Ok(config)
    }

    fn environment_override(&self) -> Option<Environment> {
        self.environment.clone()
    }
}

fn deser_string_or_i32<'de, D>(deserializer: D) -> Result<i32, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    struct StringOrI32(std::marker::PhantomData<fn() -> i32>);

    impl<'de> serde::de::Visitor<'de> for StringOrI32 {
        type Value = i32;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("string or i32")
        }

        fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_i32(i32::from(v))
        }

        fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_i32(i32::from(v))
        }

        fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(v)
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            i32::try_from(v).map_err(|err| {
                serde::de::Error::custom(format_args!(
                    "failed to convert i32 from i64({v}): {err:?}"
                ))
            })
        }

        fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            i32::try_from(v).map_err(|err| {
                serde::de::Error::custom(format_args!(
                    "failed to convert i32 from i128({v}): {err:?}"
                ))
            })
        }

        fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_i32(i32::from(v))
        }

        fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_i32(i32::from(v))
        }

        fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            i32::try_from(v).map_err(|err| {
                serde::de::Error::custom(format_args!(
                    "failed to convert i32 from u32({v}): {err:?}"
                ))
            })
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            i32::try_from(v).map_err(|err| {
                serde::de::Error::custom(format_args!(
                    "failed to convert i32 from u64({v}): {err:?}"
                ))
            })
        }

        fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            i32::try_from(v).map_err(|err| {
                serde::de::Error::custom(format_args!(
                    "failed to convert i32 from u128({v}): {err:?}"
                ))
            })
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            std::str::FromStr::from_str(v).map_err(|err| {
                serde::de::Error::custom(format_args!("failed to parse i32 from {v}: {err:?}"))
            })
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            std::str::FromStr::from_str(v.as_str()).map_err(|err| {
                serde::de::Error::custom(format_args!("failed to parse i32 from {v}: {err:?}"))
            })
        }
    }

    deserializer.deserialize_any(StringOrI32(std::marker::PhantomData))
}
