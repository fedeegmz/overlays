//! AI provider adapters (D1). Only Anthropic ships in this change; the
//! `AiProvider` port keeps the door open for more providers.

pub mod anthropic;

/// The fixed system prompt every generation uses (overlay contract). Embedded
/// at compile time from the repo root — the file lives next to the docs.
pub const BASE_PROMPT: &str = include_str!("../../../../prompt-base-overlays.md");
