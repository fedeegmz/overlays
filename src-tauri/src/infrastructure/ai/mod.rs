//! AI provider adapters (D1). Only Anthropic ships in this change; the
//! `AiProvider` port keeps the door open for more providers.

pub mod anthropic;

// Consumed by the generation service; wired later in PR3.
#[allow(dead_code)]
pub const BASE_PROMPT: &str = include_str!("../../../../prompt-base-overlays.md");
