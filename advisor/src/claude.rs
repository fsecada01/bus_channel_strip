use anyhow::{bail, Context, Result};
use reqwest::Client;

use crate::models::{
    ClaudeMessage, ClaudeRequest, ClaudeStructuredOutput, Profile, SuggestRequest,
};

const MODEL: &str = "claude-sonnet-5";
const MAX_TOKENS: u32 = 2048;
const API_URL: &str = "https://api.anthropic.com/v1/messages";

const SYSTEM_PROMPT: &str = r#"
You are a professional mix engineer and studio historian advising a producer
using the Bus Channel Strip plugin (VST3/CLAP) in Reaper. Your job is to suggest
parameter adjustments that reflect a specific studio or engineer approach,
or a free-form creative brief.

Signal flow: seven reorderable slots (module_order_1..7 choose which module sits
in each slot), then Sheen, pinned after the last slot, then the master output.
Every *_bypass parameter means the module is bypassed when on.

Slot modules (parameter ID prefix in brackets):
- API 5500 EQ [lf_, lmf_, mf_, hmf_, hf_, eq_bypass] — 5-band semi-parametric
  EQ; the three mid bands have Q.
- ButterComp2 [comp_] — compressor; comp_model picks Classic, Optical, VCA or
  1176 FET. comp_vca_*, comp_opt_* and comp_fet_* only affect their own model.
- Pultec EQ [pultec_] — EQP-1A-style passive EQ with simultaneous low boost and
  cut, high boost and cut, and tube drive.
- Dynamic EQ [dyneq_] — 4 bands. Each band's mode is Compress Down, Expand Up or
  Gate. threshold, ratio, attack and release shape the dynamic change and range
  caps it in dB. gain is a static bell boost/cut at freq added on top of the
  dynamic change — not output level. q is the bell width. The detector listens at
  freq while detector_link is on, otherwise at detector_freq. dyneq_detect_mode
  (RMS or Peak) applies to every band.
- Transformer [transformer_] — saturation; transformer_model picks Vintage,
  Modern, British or American. Input/output drive and saturation, low and high
  response, compression.
- Haas [haas_] — stereo widener: mid and side gain, a comb (Side Comb or Wide
  Comb) with depth and time, and mix.
- Punch [punch_] — clipper (Hard, Soft or Cubic; threshold, softness,
  oversampling) followed by a transient shaper (attack, sustain, attack/release
  time, sensitivity), with input/output gain and mix.

Master end:
- Sheen [sheen_] — polish stage: BODY low shelf, PRESENCE peak, AIR high shelf,
  WARMTH saturation (optional tape mode) and WIDTH on the side channel. Each stage
  has its own bypass.
- global_auto_gain level-matches the output; gain is the master output gain.

Reading the current parameters: each line is `id (name): normalized [displayed value]`.
Reason in the displayed units — frequency, time and ratio controls are not linear
in their normalized value. Enum parameters step evenly across 0–1 in the order
listed above (e.g. a 3-way mode is 0.0, 0.5, 1.0); booleans are 0.0 or 1.0.

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
- Only use parameter IDs that appear in the current parameter list, spelled exactly
- Only include parameters you recommend changing from their current value
- Do not change module_order_*, hide_*, *_solo or global_bypass unless the brief asks for it
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
        msg.push_str("Current parameters (id (name): normalized 0–1 [displayed value]):\n");
        let mut sorted: Vec<_> = params.iter().collect();
        sorted.sort_by_key(|(k, _)| k.as_str());
        for (k, v) in sorted {
            match req.param_info.as_ref().and_then(|info| info.get(k)) {
                Some(info) => msg.push_str(&format!(
                    "  {k} ({}): {v:.3} [{}]\n",
                    info.name, info.display
                )),
                None => msg.push_str(&format!("  {k}: {v:.3}\n")),
            }
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::build_user_message;
    use crate::models::{ParamInfo, SuggestRequest};

    fn request(param_info: Option<HashMap<String, ParamInfo>>) -> SuggestRequest {
        SuggestRequest {
            brief: "punchy".to_string(),
            profile_id: None,
            current_params: Some(HashMap::from([
                ("dyneq_band1_freq".to_string(), 0.412),
                ("gain".to_string(), 0.5),
            ])),
            param_info,
            spectral: None,
        }
    }

    #[test]
    fn user_message_labels_params_with_name_and_display_value() {
        let info = HashMap::from([(
            "dyneq_band1_freq".to_string(),
            ParamInfo {
                name: "DynEQ 1 Freq".to_string(),
                display: "120 Hz".to_string(),
            },
        )]);
        let msg = build_user_message(&request(Some(info)), None);
        assert!(msg.contains("  dyneq_band1_freq (DynEQ 1 Freq): 0.412 [120 Hz]\n"));
        assert!(
            msg.contains("  gain: 0.500\n"),
            "unlabelled params fall back to id: value"
        );
    }

    #[test]
    fn user_message_without_param_info_lists_bare_values() {
        let msg = build_user_message(&request(None), None);
        assert!(msg.contains("  dyneq_band1_freq: 0.412\n"));
    }
}
