use hocon::HoconLoader;
use serde::Deserialize;
use std::error::Error;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub domain: DomainConfig,
    pub runs: RunConfig,
}

#[derive(Debug, Deserialize)]
pub struct DomainConfig {
    #[serde(default)]
    pub file: Option<String>,

    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub fixed: Option<String>,

    #[serde(default = "DomainConfig::default_repeat")]
    pub repeat: u8,
}

impl DomainConfig {
    fn default_repeat() -> u8 {
        1
    }
}

#[derive(Debug, Deserialize)]
pub struct RunConfig {
    #[serde(alias = "DNS")]
    pub dns: ServerListConfig,
    #[serde(alias = "DoT")]
    pub dot: ServerListConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerListConfig {
    pub servers: Vec<String>,
}

impl AppConfig {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let config_file = Path::new("config/application.conf");
        let conf: AppConfig = HoconLoader::new().load_file(config_file)?.resolve()?;
        Ok(conf)
    }
}
