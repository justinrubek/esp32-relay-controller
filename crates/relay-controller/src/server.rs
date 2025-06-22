use crate::error::Result;
use crate::relay::RelayController;
use crate::wifi::WifiState;
use axum::{
    extract::{Path, State},
    routing::*,
    Json, Router,
};
use log::info;
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::{IpAddr, Ipv4Addr};
use std::{net::SocketAddr, sync::Arc};

#[derive(Debug, Deserialize)]
struct RelayAction {
    state: Option<bool>,
}

struct SharedState {
    #[allow(dead_code)]
    pub wifi_state: Arc<WifiState>,
    pub relay_controller: Arc<RelayController>,
}

pub async fn run_server(
    wifi_state: Arc<WifiState>,
    relay_controller: Arc<RelayController>,
) -> Result<()> {
    let state = Arc::new(SharedState {
        wifi_state,
        relay_controller,
    });

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 80);
    let app = Router::new()
        .route("/", get(move || async { "Hello!" }))
        .route("/relays", get(get_all_relays))
        .route("/relays/{id}", get(get_relay_state).put(set_relay_state))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("API server listening on {addr:?}");
    Ok(axum::serve(listener, app.into_make_service()).await?)
}

async fn get_relay_state(
    State(state): State<Arc<SharedState>>,
    Path(relay_id): Path<usize>,
) -> Json<Value> {
    let relay_status = state.relay_controller.get_state(relay_id);

    match relay_status {
        Some(active) => Json(json!({
            "relay_id": relay_id,
            "active": active,
            "success": true
        })),
        None => Json(json!({
            "relay_id": relay_id,
            "success": false,
            "error": "Invalid relay ID or failed to read state"
        })),
    }
}

async fn get_all_relays(State(state): State<Arc<SharedState>>) -> Json<Value> {
    let relay_states = state.relay_controller.get_all_states();

    let relays = relay_states
        .into_iter()
        .map(|(id, active)| {
            json!({
                "relay_id": id,
                "active": active
            })
        })
        .collect::<Vec<_>>();

    Json(json!({
        "relays": relays,
        "count": relays.len(),
        "success": true
    }))
}

async fn set_relay_state(
    State(state): State<Arc<SharedState>>,
    Path(relay_id): Path<usize>,
    Json(action): Json<RelayAction>,
) -> Json<Value> {
    let result = match action.state {
        Some(new_state) => state.relay_controller.set_state(relay_id, new_state),
        None => state.relay_controller.toggle(relay_id),
    };

    match result {
        Some(previous_state) => {
            let current_state = state.relay_controller.get_state(relay_id);
            Json(json!({
                "active": current_state,
                "previous_state": previous_state,
                "success": true
            }))
        }
        None => Json(json!({
            "relay_id": relay_id,
            "success": false,
            "error": "Invalid relay ID or failed to set state"
        })),
    }
}
