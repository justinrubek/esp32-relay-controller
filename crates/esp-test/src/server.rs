use crate::error::Result;
use crate::wifi::WifiState;
use axum::{extract::State, routing::*, Json, Router};
use log::info;
use serde_json::{json, Value};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::atomic::AtomicIsize;
use std::{net::SocketAddr, sync::Arc};

// Shared state that all Axum handlers can access
struct SharedState {
    pub counter: AtomicIsize,
    pub wifi_state: Arc<WifiState>,
}

pub async fn run_server(wifi_state: Arc<WifiState>) -> Result<()> {
    let state = Arc::new(SharedState {
        counter: AtomicIsize::new(0),
        wifi_state,
    });

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 80);
    let app = Router::new()
        .route("/", get(move || async { "Hello!" }))
        .route("/state", get(get_state))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("API server listening on {addr:?}");
    Ok(axum::serve(listener, app.into_make_service()).await?)
}

// Handler for the /state route
async fn get_state(State(state): State<Arc<SharedState>>) -> Json<Value> {
    let ip = state.wifi_state.ip_addr().await;
    let mac = state.wifi_state.mac_address.clone();
    let counter = state
        .counter
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    Json(json!({
        "message": "hello from esp32",
        "free_heap": unsafe { esp_idf_sys::esp_get_free_heap_size() },
        "ip_address": ip,
        "mac_address": mac,
        "counter": counter,
    }))
}
