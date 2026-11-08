//! Frevo server binary.
//!
//! Phase 0 placeholder. It serves exactly one real endpoint, `/fhir/metadata`,
//! so that the container image, release pipeline and deployment manifests can be
//! proven end to end before any FHIR behaviour exists. Every further endpoint
//! arrives through an OpenSpec change.
#![forbid(unsafe_code)]

use axum::{Json, Router, http::StatusCode, routing::get};
use serde_json::{Value, json};

/// Software version reported in the CapabilityStatement.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Minimal, deliberately honest CapabilityStatement: it advertises no resources,
/// because none are implemented yet.
fn capability_statement() -> Value {
    json!({
        "resourceType": "CapabilityStatement",
        "status": "active",
        "date": "2026-11-08",
        "publisher": "The Frevo authors",
        "kind": "instance",
        "software": { "name": "Frevo", "version": VERSION },
        "implementation": {
            "description": "Frevo - a high-performance HL7 FHIR server in Rust (bootstrap build, no FHIR capabilities enabled)"
        },
        "fhirVersion": "4.0.1",
        "format": ["application/fhir+json"],
        "rest": [{ "mode": "server", "resource": [] }]
    })
}

async fn metadata() -> (StatusCode, [(&'static str, &'static str); 1], Json<Value>) {
    (
        StatusCode::OK,
        [("content-type", "application/fhir+json; charset=utf-8")],
        Json(capability_statement()),
    )
}

async fn healthz() -> &'static str {
    "ok"
}

fn app() -> Router {
    Router::new()
        .route("/fhir/metadata", get(metadata))
        .route("/healthz", get(healthz))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let port: u16 = std::env::var("FREVO_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, version = VERSION, "frevo listening");
    axum::serve(listener, app())
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// covers: server.metadata.001
    #[test]
    fn capability_statement_is_a_capability_statement() {
        let cs = capability_statement();
        assert_eq!(cs["resourceType"], "CapabilityStatement");
        assert_eq!(cs["status"], "active");
        assert_eq!(cs["fhirVersion"], "4.0.1");
        assert_eq!(cs["software"]["name"], "Frevo");
    }

    /// covers: server.metadata.002
    #[test]
    fn bootstrap_build_advertises_no_resources() {
        let cs = capability_statement();
        assert_eq!(cs["rest"][0]["mode"], "server");
        assert_eq!(
            cs["rest"][0]["resource"].as_array().map(Vec::len),
            Some(0),
            "the bootstrap build must not claim capabilities it does not have"
        );
    }
}
