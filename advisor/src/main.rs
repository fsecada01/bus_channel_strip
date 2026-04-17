mod claude;
mod models;
mod routes;

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use axum::{
    routing::{get, post},
    Router,
};
use models::{Profile, ProfilesFile};
use tower::ServiceBuilder;
use tower_http::cors::{Any, CorsLayer};
use tower_http::normalize_path::NormalizePathLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// ── Shared state ──────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub profiles: Arc<Vec<Profile>>,
    pub api_key: Arc<String>,
    pub client: reqwest::Client,
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // Tracing — respects RUST_LOG env var; defaults to info
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Config from environment
    let port: u16 = std::env::var("BCS_ADVISOR_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(7373);

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .context("ANTHROPIC_API_KEY environment variable not set")?;

    // Profiles path: BCS_PROFILES_PATH env var or default relative to workspace root
    let profiles_path = std::env::var("BCS_PROFILES_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            // When running from workspace root or advisor/ directory, resolve correctly
            let mut p = std::env::current_dir().unwrap_or_default();
            // If we're inside the advisor/ subdirectory, step up
            if p.ends_with("advisor") {
                p.pop();
            }
            p.join("docs/specs/studio-profiles.json")
        });

    info!("Loading profiles from {}", profiles_path.display());
    let profiles = load_profiles(&profiles_path)
        .with_context(|| format!("Failed to load profiles from {}", profiles_path.display()))?;
    info!("Loaded {} profiles", profiles.len());

    let state = AppState {
        profiles: Arc::new(profiles),
        api_key: Arc::new(api_key),
        client: reqwest::Client::new(),
    };

    // Router
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/profiles", get(routes::profiles::list_profiles))
        .route("/profiles/:id", get(routes::profiles::get_profile))
        .route("/suggest", post(routes::suggest::suggest))
        .route("/health", get(health))
        .with_state(state)
        .layer(
            ServiceBuilder::new()
                .layer(NormalizePathLayer::trim_trailing_slash())
                .layer(cors),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("BCS Mix Advisor listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

fn load_profiles(path: &PathBuf) -> Result<Vec<Profile>> {
    let data = std::fs::read_to_string(path)?;
    let file: ProfilesFile = serde_json::from_str(&data)?;
    Ok(file.profiles)
}
