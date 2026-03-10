use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::{models::{Profile, ProfileSummary}, AppState};

/// GET /profiles — list all profiles (id, display_name, entity_type, era, genre_gates, description)
pub async fn list_profiles(
    State(state): State<AppState>,
) -> Json<Vec<ProfileSummary>> {
    let summaries = state
        .profiles
        .iter()
        .map(|p| ProfileSummary {
            id: p.id.clone(),
            display_name: p.display_name.clone(),
            entity_type: p.entity_type.clone(),
            era: p.era.clone(),
            genre_gates: p.genre_gates.clone(),
            description: p.description.clone(),
        })
        .collect();

    Json(summaries)
}

/// GET /profiles/:id — full profile JSON
pub async fn get_profile(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Profile>, StatusCode> {
    state
        .profiles
        .iter()
        .find(|p| p.id == id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}
