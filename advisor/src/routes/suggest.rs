use axum::{extract::State, http::StatusCode, Json};

use crate::{
    claude,
    models::{SuggestRequest, SuggestResponse},
    AppState,
};

/// POST /suggest — generate parameter suggestions via Claude
pub async fn suggest(
    State(state): State<AppState>,
    Json(req): Json<SuggestRequest>,
) -> Result<Json<SuggestResponse>, (StatusCode, String)> {
    // Resolve profile if requested
    let profile = req
        .profile_id
        .as_deref()
        .and_then(|id| state.profiles.iter().find(|p| p.id == id).cloned());

    if req.profile_id.is_some() && profile.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Profile '{}' not found", req.profile_id.unwrap()),
        ));
    }

    // Call Claude
    let result = claude::suggest(&state.api_key, &state.client, &req, profile.as_ref())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SuggestResponse {
        summary: result.summary,
        parameters: result.parameters,
        rationale: result.rationale,
        warnings: result.warnings,
        profile_used: profile.map(|p| p.id),
    }))
}
