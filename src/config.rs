use hocon::HoconLoader;
use serde::Deserialize;
use std::error::Error;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub domain: DomainConfig,
    pub groups: Vec<GroupConfig>,
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
    const fn default_repeat() -> u8 {
        1
    }
}

#[derive(Debug, Deserialize)]
pub struct GroupConfig {
    pub name: String,
    #[serde(default)]
    pub ip4: Vec<String>,
    #[serde(default)]
    pub ip6: Vec<String>,
    #[serde(default, alias = "DoT")]
    pub dot: Vec<String>,
}

impl AppConfig {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let config_file = Path::new("config/application.conf");
        if !config_file.exists() {
            Err(format!("{} does not exist", config_file.display()))?
        } else {
            let conf: AppConfig = HoconLoader::new().load_file(config_file)?.resolve()?;
            Ok(conf)
        }
    }
}
