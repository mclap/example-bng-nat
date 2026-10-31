use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub ports: Vec<Port>,
    #[serde(default)]
    pub cgnat: Cgnat,
    #[serde(default)]
    pub routes: Vec<RouteConfig>,
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let cfg: Config = toml::from_str(&content)?;
        Ok(cfg)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Port {
    pub name: String,
    #[serde(default)]
    pub role: PortRole,
    #[serde(default)]
    pub subnet: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortRole {
    #[default]
    Subscriber,
    Uplink,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Cgnat {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub listen_subnet: Option<String>,
    #[serde(default)]
    pub public_pool: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RouteConfig {
    pub dst: String,
    pub port: String,
}
