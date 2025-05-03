use econfmanager::interface::InterfaceInstance;
use std::sync::{Arc, Mutex};
use warp::{Filter, Rejection, Reply, ws::{Message, WebSocket}};
use serde::{Deserialize, Serialize};
use futures::{SinkExt, StreamExt};

#[derive(Serialize, Deserialize, Clone)]
struct Parameter {
    name: String,
    value: serde_json::Value,
}

#[derive(Default)]
struct AppState {
    string_param: String,
    int_param: i32,
    subscribers: Vec<tokio::sync::mpsc::UnboundedSender<String>>,
    interface: InterfaceInstance,
}

type SharedState = Arc<Mutex<AppState>>;

#[tokio::main]
async fn main() {

    let database_path = "".to_string();
    let saved_database_path = "".to_string();

    let state = Arc::new(Mutex::new(AppState {
        string_param: "Hello".into(),
        int_param: 42,
        subscribers: Vec::new(),
        interface: InterfaceInstance::new(&database_path, &saved_database_path).unwrap(),
    }));

    let state_filter = warp::any().map(move || state.clone());

    let json_rpc = warp::post()
        .and(warp::path("rpc"))
        .and(warp::body::json())
        .and(state_filter.clone())
        .and_then(handle_rpc);

    let ws = warp::path("ws")
        .and(warp::ws())
        .and(state_filter.clone())
        .map(|ws: warp::ws::Ws, state| {
            ws.on_upgrade(move |socket| handle_ws(socket, state))
        });

    let routes = json_rpc.or(ws);
    println!("Listening on http://localhost:3030");

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

#[derive(Deserialize)]
struct RpcRequest {
    id: serde_json::Value,
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct RpcResponse {
    id: serde_json::Value,
    result: serde_json::Value,
}

async fn handle_rpc(req: RpcRequest, state: SharedState) -> Result<impl Reply, Rejection> {
    let mut app = state.lock().unwrap();
    let result = match req.method.as_str() {
        "read" => {
            let name = req.params.as_ref().and_then(|p| p.get("name")).and_then(|v| v.as_str()).unwrap_or("");
            match name {
                "int_param" => serde_json::json!({"name": "int_param", "value": app.int_param}),
                "string_param" => serde_json::json!({"name": "string_param", "value": app.string_param}),
                _ => serde_json::json!({"error": "Unknown parameter"}),
            }
        }
        "write" => {
            if let Some(params) = req.params.as_ref() {
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if name == "int_param" {
                    if let Some(value) = params.get("value").and_then(|v| v.as_i64()) {
                        app.int_param = value as i32;

                        let notification = serde_json::json!({
                            "jsonrpc": "2.0",
                            "method": "notify",
                            "params": {
                                "name": "int_param",
                                "value": app.int_param
                            }
                        })
                        .to_string();

                        app.subscribers.retain(|tx| tx.send(notification.clone()).is_ok());
                    }
                }
            }
            serde_json::json!({"status": "ok"})
        }
        _ => serde_json::json!({"error": "Unknown method"}),
    };

    Ok(warp::reply::json(&RpcResponse {
        id: req.id,
        result,
    }))
}

async fn handle_ws(ws: WebSocket, state: SharedState) {
    let (mut tx, mut rx) = ws.split();
    let (client_tx, mut client_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    {
        let mut app = state.lock().unwrap();
        app.subscribers.push(client_tx);
    }

    tokio::spawn(async move {
        while let Some(msg) = client_rx.recv().await {
            let _ = tx.send(Message::text(msg)).await;
        }
    });

    while rx.next().await.is_some() {
        // ignore client messages
    }
}
