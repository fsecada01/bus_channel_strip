use anyhow::{bail, Context, Result};
use reqwest::Client;

use crate::models::{
    ClaudeMessage, ClaudeRequest, ClaudeStructuredOutput, Profile, SuggestRequest,
};

const MODEL: &str = "claude-sonnet-4-6";
const MAX_TOKENS: u32 = 1024;
const API_URL: &str = "https://api.anthropic.com/v1/messages";

const SYSTEM_PROMPT: &str = r#"
You are a professional mix engineer and studio historian advising a producer
using the Bus Channel Strip VST3 plugin in Reaper. Your job is to suggest
parameter adjustments that reflect a specific studio or engineer approach,
or a free-form creative brief.

The plugin has six modules in signal order:
1. api5500      — 5-band semi-parametric EQ (API 550A-style)
2. buttercomp2  — Compressor (VCA / Optical / FET / Tube models)
3. pultec       — Passive EQ (simultaneous boost+cut, Pultec EQP-1A style)
4. dynamic_eq   — 4-band frequency-dependent compressor
5. transformer  — Harmonic saturation (4 vintage models: Trident/API/Neve/SSL)
6. punch        — Clipper + transient shaper

You MUST respond with a JSON object and nothing else — no markdown, no prose outside the object.
The JSON must have exactly these keys:

{
  "summary": "2-3 sentence description of the approach and sonic character",
  "parameters": {
    "<param_id>": <normalized_value_0_to_1>,
    ...
  },
  "rationale": {
    "<param_id>": "brief explanation",
    ...
  },
  "warnings": ["optional string", ...]
}

Rules:
- Only include parameters you recommend changing from their current value
- All parameter values must be normalized floats in [0.0, 1.0]
- Be specific: 0.55 is better than 0.5 when you have a reason
- Do not suggest cosmetic or neutral changes
- If the brief conflicts with the profile, note it in warnings
- Keep rationale strings under 15 words each
"#;

pub async fn suggest(
    api_key: &str,
    client: &Client,
    req: &SuggestRequest,
    profile: Option<&Profile>,
) -> Result<ClaudeStructuredOutput> {
    let user_content = build_user_message(req, profile);

    let body = ClaudeRequest {
        model: MODEL.to_string(),
        max_tokens: MAX_TOKENS,
        system: SYSTEM_PROMPT.trim().to_string(),
        messages: vec![ClaudeMessage {
            role: "user".to_string(),
            content: user_content,
        }],
    };

    let response = client
        .post(API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .context("HTTP request to Claude API failed")?;

    let status = response.status();
    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        bail!("Claude API returned {status}: {text}");
    }

    let claude_resp = response
        .json::<crate::models::ClaudeResponse>()
        .await
        .context("Failed to parse Claude API response")?;

    let text = claude_resp
        .content
        .into_iter()
        .find(|c| c.content_type == "text")
        .and_then(|c| c.text)
        .context("Claude response contained no text block")?;

    // Claude should return raw JSON — strip any accidental markdown fences
    let json_text = strip_fences(&text);

    serde_json::from_str::<ClaudeStructuredOutput>(json_text)
        .context("Claude response was not valid structured JSON")
}

fn build_user_message(req: &SuggestRequest, profile: Option<&Profile>) -> String {
    let mut msg = format!("Creative brief: {}\n\n", req.brief);

    if let Some(params) = &req.current_params {
        msg.push_str("Current parameter values (normalized 0–1):\n");
        let mut sorted: Vec<_> = params.iter().collect();
        sorted.sort_by_key(|(k, _)| k.as_str());
        for (k, v) in sorted {
            msg.push_str(&format!("  {k}: {v:.3}\n"));
        }
        msg.push('\n');
    }

    if let Some(sp) = req.spectral {
        msg.push_str(&format!(
            "Real-time spectral analysis (band energy 0–1):\n  Sub/Low: {:.2}  Low-Mid: {:.2}  Hi-Mid: {:.2}  High: {:.2}\n\n",
            sp[0], sp[1], sp[2], sp[3]
        ));
    }

    if let Some(p) = profile {
        msg.push_str("Studio/Engineer Profile:\n");
        if let Ok(json) = serde_json::to_string_pretty(&p.extra) {
            msg.push_str(&json);
        }
        msg.push('\n');
    }

    msg
}

fn strip_fences(s: &str) -> &str {
    let s = s.trim();
    let s = s.strip_prefix("```json").unwrap_or(s);
    let s = s.strip_prefix("```").unwrap_or(s);
    let s = s.strip_suffix("```").unwrap_or(s);
    s.trim()
}
