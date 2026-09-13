//! AI provider adapters (D1). Only Anthropic ships in this change; the
//! `AiProvider` port keeps the door open for more providers.

pub mod anthropic;

/// The fixed system prompt every generation uses (overlay contract). Embedded
/// at compile time from the repo root — the file lives next to the docs.
pub const BASE_PROMPT: &str = include_str!("../../../../prompt-base-overlays.md");

/// Exact JSON shape the model must answer with (fences, prose and anything
/// else are rejected downstream). Kept separate from the base prompt so the
/// output contract can evolve independently.
pub const OUTPUT_SHAPE: &str = include_str!("../../../../prompt-output-shape.md");

/// Full system prompt: base contract + output shape. Composed here in the
/// infrastructure layer and injected into the generation service — the
/// application layer never imports prompt constants.
pub fn system_prompt() -> String {
    format!("{BASE_PROMPT}\n\n{OUTPUT_SHAPE}")
}
