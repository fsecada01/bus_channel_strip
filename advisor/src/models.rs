use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Studio Profile (mirrors studio-profiles-schema.md) ────────────────────────

/// Lightweight listing entry for GET /profiles
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProfileSummary {
    pub id: String,
    pub display_name: String,
    pub entity_type: String,
    pub era: String,
    pub genre_gates: Vec<String>,
    pub description: String,
}

/// Full profile as stored in studio-profiles.json
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Profile {
    pub id: String,
    pub display_name: String,
    pub entity_type: String,
    pub era: String,
    pub genre_gates: Vec<String>,
    pub description: String,
    // All remaining fields passed through opaquely for Claude context
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

/// Top-level structure of studio-profiles.json
#[derive(Debug, Deserialize)]
pub struct ProfilesFile {
    pub profiles: Vec<Profile>,
}

// ── Suggest API ───────────────────────────────────────────────────────────────

/// POST /suggest request body
#[derive(Debug, Deserialize)]
pub struct SuggestRequest {
    /// Free-form creative brief (required)
    pub brief: String,

    /// Optional profile ID to anchor the suggestion
    pub profile_id: Option<String>,

    /// Current plugin parameter values, normalized 0.0–1.0
    /// Keys are NIH-plug stable parameter IDs
    pub current_params: Option<HashMap<String, f32>>,

    /// Optional real-time spectral band energy [sub_low, low_mid, hi_mid, high]
    pub spectral: Option<[f32; 4]>,
}

/// POST /suggest response
#[derive(Debug, Serialize)]
pub struct SuggestResponse {
    /// 2-3 sentence narrative of the approach
    pub summary: String,

    /// Suggested parameter values, normalized 0.0–1.0
    /// Only parameters the advisor recommends changing
    pub parameters: HashMap<String, f32>,

    /// Per-parameter explanation keyed by parameter ID
    pub rationale: HashMap<String, String>,

    /// Potential issues or conflicts to watch for
    pub warnings: Vec<String>,

    /// Which profile was used (null if none)
    pub profile_used: Option<String>,
}

/// Claude API message format
#[derive(Debug, Serialize)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: String,
}

/// Claude API request body
#[derive(Debug, Serialize)]
pub struct ClaudeRequest {
    pub model: String,
    pub max_tokens: u32,
    pub system: String,
    pub messages: Vec<ClaudeMessage>,
}

/// Claude API response (partial — only fields we use)
#[derive(Debug, Deserialize)]
pub struct ClaudeResponse {
    pub content: Vec<ClaudeContent>,
}

#[derive(Debug, Deserialize)]
pub struct ClaudeContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
}

/// The structured JSON Claude is asked to return inside its text response
#[derive(Debug, Deserialize)]
pub struct ClaudeStructuredOutput {
    pub summary: String,
    pub parameters: HashMap<String, f32>,
    pub rationale: HashMap<String, String>,
    pub warnings: Vec<String>,
}
