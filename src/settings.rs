use serde::Deserialize;
use settings_loader::common::database::DatabaseSettings;
use settings_loader::common::http::HttpServerSettings;
use settings_loader::SettingsLoader;

mod cli_options;
mod http_api_settings;
#[cfg(test)]
mod tests;

pub use cli_options::CliOptions;
pub use http_api_settings::HttpApiSettings;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Settings {
    pub http_api: HttpServerSettings,
    pub database: DatabaseSettings,

    #[serde(flatten)]
    pub correlation: CorrelationSettings,
}

impl SettingsLoader for Settings {
    type Options = CliOptions;
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize)]
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
