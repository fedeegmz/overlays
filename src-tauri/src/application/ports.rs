use std::path::Path;

use crate::domain::ai::{AiError, AiText, ProviderKind};
use crate::domain::config::AppConfig;
use crate::domain::error::DomainResult;
use crate::domain::overlay::OverlayPayload;
use crate::domain::preset::Preset;
use crate::domain::template::TemplateInfo;

pub trait PresetRepository: Send + Sync {
    fn load(&self) -> Vec<Preset>;
    fn save(&self, presets: &[Preset]) -> DomainResult<()>;
}

pub trait ConfigRepository: Send + Sync {
    fn load(&self) -> AppConfig;
    fn save(&self, config: &AppConfig) -> DomainResult<()>;
}

pub trait TemplateSource: Send + Sync {
    fn discover(&self, overlay_dir: &Path) -> DomainResult<Vec<TemplateInfo>>;
}

pub trait OverlayBus: Send + Sync {
    fn publish(&self, payload: &OverlayPayload);
    fn snapshot(&self) -> Vec<OverlayPayload>;
}

/// OS keyring access. Secrets never touch config.json, State, logs, or error
/// params (K1/K4) — only presence metadata crosses IPC.
// Consumed by keyring infra + commands; wired in PR2.
#[allow(dead_code)]
pub trait KeyStore: Send + Sync {
    fn set(&self, provider: &str, secret: &str) -> DomainResult<()>;
    fn get(&self, provider: &str) -> DomainResult<String>;
    fn delete(&self, provider: &str) -> DomainResult<()>;
}

/// AI provider adapter (D1). `generate` returns raw text plus a truncated
/// flag (`stop_reason == "max_tokens"`); normalization happens in the service.
// Consumed by generation service; wired in PR3.
#[allow(dead_code)]
pub trait AiProvider: Send + Sync {
    fn kind(&self) -> ProviderKind;
    fn models(&self) -> Vec<String>;
    fn generate(
        &self,
        model: &str,
        prompt: &str,
        system: &str,
        key: &str,
    ) -> Result<AiText, AiError>;
}
