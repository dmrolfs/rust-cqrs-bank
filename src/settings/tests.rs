use super::*;
pub use tokio_test::assert_ok;
pub use trim_margin::MarginTrimmable;

mod loading {
    use super::*;
    use crate::settings::http_api_settings::RateLimitSettings;
    use pretty_assertions::assert_eq;
    use secrecy::Secret;
    use settings_loader::common::http::HttpServerSettings;
    use std::time::Duration;

    static SETTINGS: once_cell::sync::Lazy<Settings> = once_cell::sync::Lazy::new(|| Settings {
        http_api: HttpApiSettings {
            server: HttpServerSettings { host: "0.0.0.0".to_string(), port: 8000 },
            timeout: Duration::from_secs(2 * 60),
            rate_limit: RateLimitSettings {
                nr_requests: 100,
                per_duration: Duration::from_secs(60),
            },
        },
        database: DatabaseSettings {
            username: "otis".to_string(),
            password: Secret::new("neo".to_string()),
            host: "localhost".to_string(),
            port: 5432,
            database_name: "bank".to_string(),
            require_ssl: true,
            min_connections: None,
            max_connections: Some(10),
            idle_timeout: Some(Duration::from_secs(300)),
            max_lifetime: Some(Duration::from_secs(1_800)),
        },
        correlation: CorrelationSettings::default(),
    });

    #[test]
    fn test_settings_serde_roundtrip() {
        let yaml = r##"|---
            |http_api:
            |  timeout_secs: 300
            |  host: 0.0.0.0
            |  port: 8000
            |  rate_limit:
            |    nr_requests: 100
            |    per_secs: 60
            |database:
            |  username: user_1
            |  password: my_password
            |  host: 0.0.0.0
            |  port: 1234
            |  database_name: my_database
            |  require_ssl: true
            |  max_connections: 10
            |  idle_timeout_secs: 300
            |machine_id: 1
            |node_id: 1
            |"##
        .trim_margin()
        .unwrap();

        let expected = Settings {
            http_api: HttpApiSettings {
                server: HttpServerSettings { host: "0.0.0.0".to_string(), port: 8000 },
                timeout: Duration::from_secs(300),
                rate_limit: RateLimitSettings {
                    nr_requests: 100,
                    per_duration: Duration::from_secs(60),
                },
            },
            database: DatabaseSettings {
                username: "user_1".to_string(),
                password: Secret::new("my_password".to_string()),
                host: "0.0.0.0".to_string(),
                port: 1234,
                database_name: "my_database".to_string(),
                require_ssl: true,
                min_connections: None,
                max_connections: Some(10),
                idle_timeout: Some(Duration::from_secs(300)),
                max_lifetime: None,
            },
            correlation: CorrelationSettings { machine_id: 1, node_id: 1 },
        };

        let actual: Settings = assert_ok!(serde_yaml::from_str(&yaml));
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_basic_load() {
        let c = assert_ok!(config::Config::builder()
            .add_source(config::File::from(std::path::PathBuf::from(
                "./tests/data/settings.yaml"
            )))
            .build());

        let actual: Settings = assert_ok!(c.try_deserialize());

        let expected = Settings {
            http_api: HttpApiSettings {
                timeout: Duration::from_secs(300),
                ..SETTINGS.http_api.clone()
            },
            database: DatabaseSettings {
                database_name: "stella".to_string(),
                max_lifetime: None,
                ..SETTINGS.database.clone()
            },
            ..SETTINGS.clone()
        };

        assert_eq!(actual, expected);
    }

    // #[ignore]
    #[test]
    fn test_settings_applications_load() -> anyhow::Result<()> {
        once_cell::sync::Lazy::force(&crate::tracing::TEST_TRACING);
        let main_span = tracing::info_span!("test_settings_applications_load");
        let _ = main_span.enter();

        let options = CliOptions {
            settings_search_path: Some("./resources".into()),
            secrets: Some("./resources/secrets.yaml".into()),
            ..CliOptions::default()
        };

        tracing::debug!("testing environment...");

        temp_env::with_vars(
            vec![
                ("APP_ENVIRONMENT", None),
                ("APP__MACHINE_ID", Some("17")),
                ("APP__NODE_ID", Some("13")),
            ],
            || {
                let actual: Settings = assert_ok!(Settings::load(&options));
                assert_eq!(
                    actual.correlation,
                    CorrelationSettings { machine_id: 17, node_id: 13 }
                );

                let expected = Settings {
                    correlation: CorrelationSettings { machine_id: 17, node_id: 13 },
                    database: DatabaseSettings {
                        username: "postgres".to_string(),
                        password: Secret::new("password".to_string()),
                        database_name: "bank".to_string(),
                        require_ssl: false,
                        ..SETTINGS.database.clone()
                    },
                    ..SETTINGS.clone()
                };

                assert_eq!(actual, expected);
            },
        );

        Ok(())
    }

    #[test]
    fn test_local_load() -> anyhow::Result<()> {
        once_cell::sync::Lazy::force(&crate::tracing::TEST_TRACING);
        let main_span = tracing::info_span!("test_local_load");
        let _ = main_span.enter();

        let options = CliOptions {
            settings_search_path: Some("./resources".into()),
            ..CliOptions::default()
        };
        // let before_env = Settings::load(&options);
        // tracing::info!("from Settings::load: {:?}", before_env);
        // let before_env = assert_ok!(before_env);
        // assert_eq!(before_env, SETTINGS.clone());

        temp_env::with_vars(
            // )
            // with_env_vars(
            //     "test_local_load",
            vec![("APP_ENVIRONMENT", Some("local"))],
            || {
                let actual: Settings = assert_ok!(Settings::load(&options));
                assert_eq!(actual.http_api.server.host.as_str(), "127.0.0.1");

                let expected = Settings {
                    http_api: HttpApiSettings {
                        server: HttpServerSettings {
                            host: "127.0.0.1".to_string(),
                            ..SETTINGS.http_api.server.clone()
                        },
                        ..SETTINGS.http_api.clone()
                    },
                    database: DatabaseSettings {
                        username: "snoopy".to_string(),
                        password: Secret::new("foobar".to_string()),
                        require_ssl: false,
                        ..SETTINGS.database.clone()
                    },
                    ..SETTINGS.clone()
                };

                tracing::debug!(?actual, ?expected, "checking local settings");
                assert_eq!(actual, expected);
            },
        );

        Ok(())
    }

    #[test]
    fn test_production_load() -> anyhow::Result<()> {
        once_cell::sync::Lazy::force(&crate::tracing::TEST_TRACING);
        let main_span = tracing::info_span!("test_production_load");
        let _ = main_span.enter();

        let options = CliOptions {
            settings_search_path: Some("./resources".into()),
            ..CliOptions::default()
        };
        // let before_env = Settings::load(&options);
        // tracing::info!("from Settings::load: {:?}", before_env);
        // let before_env = assert_ok!(before_env);
        // assert_eq!(before_env, SETTINGS.clone());

        temp_env::with_vars(
            // with_env_vars(
            //     "test_production_load",
            vec![("APP_ENVIRONMENT", Some("production"))],
            || {
                let actual: Settings = assert_ok!(Settings::load(&options));
                assert_eq!(
                    actual.correlation,
                    CorrelationSettings { machine_id: 1, node_id: 1 }
                );

                let expected = Settings {
                    correlation: CorrelationSettings { machine_id: 1, node_id: 1 },
                    database: DatabaseSettings {
                        username: "neo".to_string(),
                        host: "postgres_1632546102".to_string(),
                        password: Secret::new("pixies".to_string()),
                        require_ssl: false,
                        ..SETTINGS.database.clone()
                    },
                    ..SETTINGS.clone()
                };

                tracing::debug!(?actual, ?expected, "checking production settings");
                assert_eq!(actual, expected);
            },
        );

        Ok(())
    }
}
