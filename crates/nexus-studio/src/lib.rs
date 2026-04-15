use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tower_http::cors::CorsLayer;

// ── Data types sent to/from the SPA ─────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EdgeDto {
    pub contract: String,
    pub peer: String, // "to" for sends, "from" for receives
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeDto {
    pub name: String,
    pub sends: Vec<EdgeDto>,
    pub receives: Vec<EdgeDto>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ContractDto {
    pub name: String,
    pub transport: String,
    /// Schema path or empty string for inline-fields contracts
    pub schema: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkDto {
    pub nodes: Vec<NodeDto>,
    pub contracts: Vec<ContractDto>,
}

// ── Parse raw TOML into DTOs ──────────────────────────────────────────────────

fn toml_to_dto(toml_str: &str) -> anyhow::Result<NetworkDto> {
    let value: toml::Value = toml::from_str(toml_str)?;

    let empty_array = toml::Value::Array(vec![]);

    let nodes = value
        .get("nodes")
        .and_then(|v| v.as_array())
        .unwrap_or(empty_array.as_array().unwrap())
        .iter()
        .map(|n| {
            let name = n.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let sends = n
                .get("sends")
                .and_then(|v| v.as_array())
                .unwrap_or(empty_array.as_array().unwrap())
                .iter()
                .map(|e| EdgeDto {
                    contract: e.get("contract").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    peer: e.get("to").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                })
                .collect();
            let receives = n
                .get("receives")
                .and_then(|v| v.as_array())
                .unwrap_or(empty_array.as_array().unwrap())
                .iter()
                .map(|e| EdgeDto {
                    contract: e.get("contract").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    peer: e.get("from").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                })
                .collect();
            NodeDto { name, sends, receives }
        })
        .collect();

    let contracts = value
        .get("contracts")
        .and_then(|v| v.as_array())
        .unwrap_or(empty_array.as_array().unwrap())
        .iter()
        .map(|c| ContractDto {
            name: c.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            transport: c.get("transport").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            schema: c.get("schema").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        })
        .collect();

    Ok(NetworkDto { nodes, contracts })
}

// ── Serialise NetworkDto back to TOML ────────────────────────────────────────

fn dto_to_toml(dto: &NetworkDto) -> String {
    let mut out = String::new();

    for node in &dto.nodes {
        out.push_str("[[nodes]]\n");
        out.push_str(&format!("name = \"{}\"\n", node.name));

        if !node.sends.is_empty() {
            out.push_str("sends = [\n");
            for e in &node.sends {
                out.push_str(&format!(
                    "  {{ contract = \"{}\", to = \"{}\" }},\n",
                    e.contract, e.peer
                ));
            }
            out.push_str("]\n");
        }

        if !node.receives.is_empty() {
            out.push_str("receives = [\n");
            for e in &node.receives {
                out.push_str(&format!(
                    "  {{ contract = \"{}\", from = \"{}\" }},\n",
                    e.contract, e.peer
                ));
            }
            out.push_str("]\n");
        }

        out.push('\n');
    }

    for contract in &dto.contracts {
        out.push_str("[[contracts]]\n");
        out.push_str(&format!("name = \"{}\"\n", contract.name));
        out.push_str(&format!("transport = \"{}\"\n", contract.transport));
        if !contract.schema.is_empty() {
            out.push_str(&format!("schema = \"{}\"\n", contract.schema));
        }
        out.push('\n');
    }

    out
}

// ── Embedded SPA ──────────────────────────────────────────────────────────────

static SPA_HTML: &str = include_str!("../spa/index.html");

// ── Server state ──────────────────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    config_path: Arc<Mutex<PathBuf>>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

async fn get_network(State(state): State<AppState>) -> Result<Json<NetworkDto>, StatusCode> {
    let path = state.config_path.lock().unwrap().clone();
    let content = std::fs::read_to_string(&path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let dto = toml_to_dto(&content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(dto))
}

async fn put_network(
    State(state): State<AppState>,
    Json(dto): Json<NetworkDto>,
) -> StatusCode {
    let toml = dto_to_toml(&dto);
    let path = state.config_path.lock().unwrap().clone();
    if std::fs::write(&path, toml).is_ok() {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    }
}

async fn serve_spa() -> Response {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        SPA_HTML,
    )
        .into_response()
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub async fn run(config: PathBuf, port: u16) -> anyhow::Result<()> {
    let state = AppState {
        config_path: Arc::new(Mutex::new(config)),
    };

    let app = Router::new()
        .route("/api/network", get(get_network).put(put_network))
        .route("/", get(serve_spa))
        .route("/*path", get(serve_spa))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    eprintln!("nexus-studio listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
