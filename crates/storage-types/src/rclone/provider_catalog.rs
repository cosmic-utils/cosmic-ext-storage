use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RcloneProviderOptionExample {
    pub value: String,
    pub help: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RcloneProviderOption {
    pub name: String,
    #[serde(default)]
    pub help: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub is_password: bool,
    #[serde(default)]
    pub sensitive: bool,
    #[serde(default)]
    pub advanced: bool,
    #[serde(default)]
    pub hide: i32,
    #[serde(default, rename = "type")]
    pub value_type: String,
    #[serde(default, rename = "default")]
    pub default_value: String,
    #[serde(default)]
    pub examples: Vec<RcloneProviderOptionExample>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub section: String,
}

impl RcloneProviderOption {
    pub fn is_secure(&self) -> bool {
        self.is_password || self.sensitive
    }

    pub fn is_hidden(&self) -> bool {
        self.hide != 0
    }

    pub fn section_display_name(&self) -> &'static str {
        match self.section.as_str() {
            "authentication" => "Authentication",
            "connection" => "Connection",
            "security" => "Security",
            "storage" => "Storage",
            "transfers" => "Transfers",
            "behavior" => "Behavior",
            "other" => "Other",
            _ => "Other",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RcloneProvider {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub hide: bool,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub options: Vec<RcloneProviderOption>,
}

const RCLONE_PROVIDERS_JSON: &str =
    include_str!("../../../storage-sys/resources/rclone/providers.json");

static RCLONE_PROVIDERS: OnceLock<Vec<RcloneProvider>> = OnceLock::new();
static RCLONE_PROVIDER_TYPES: OnceLock<Vec<String>> = OnceLock::new();

pub fn rclone_providers() -> &'static [RcloneProvider] {
    RCLONE_PROVIDERS
        .get_or_init(|| serde_json::from_str(RCLONE_PROVIDERS_JSON).unwrap_or_default())
        .as_slice()
}

pub fn supported_remote_types() -> &'static [String] {
    RCLONE_PROVIDER_TYPES
        .get_or_init(|| {
            let mut types = Vec::new();
            for provider in rclone_providers() {
                types.push(provider.name.clone());
                for alias in &provider.aliases {
                    types.push(alias.clone());
                }
            }
            types.sort_by_key(|a| a.to_lowercase());
            types.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
            types
        })
        .as_slice()
}

pub fn rclone_provider(remote_type: &str) -> Option<&'static RcloneProvider> {
    let remote_type = remote_type.to_lowercase();
    rclone_providers().iter().find(|provider| {
        provider.name.eq_ignore_ascii_case(&remote_type)
            || provider
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(&remote_type))
    })
}
