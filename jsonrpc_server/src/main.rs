use arguments::Args;
use clap::Parser;
use configfile::Config;
use econfmanager::{interface::InterfaceInstance, schema::ParameterValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use warp::{Filter, Rejection, Reply, ws::{Message, WebSocket}};
use futures::{SinkExt, StreamExt};

pub mod arguments;
pub mod configfile;


#[derive(Default)]
struct AppState {
    subscribers: Vec<tokio::sync::mpsc::UnboundedSender<String>>,
    interface: InterfaceInstance,
    names: Vec<String>,
}

type SharedState = Arc<Mutex<AppState>>;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let config = Config::from_file(args.config);

    let state = Arc::new(Mutex::new(AppState {
        subscribers: Vec::new(),
        interface: InterfaceInstance::new(&config.database_path, &config.saved_database_path).unwrap(),
        names: Vec::new(),
    }));

    // Cache static parameters
    {
        let mut app = state.lock().unwrap();
        app.names = app.interface.get_parameter_names();
    }

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

fn reply_error(error: &str) -> Value {
    serde_json::json!({"error": error})
}

async fn handle_rpc(req: RpcRequest, state: SharedState) -> Result<impl Reply, Rejection> {
    let mut app = state.lock().unwrap();
    let result = match req.method.as_str() {
        "read" => {
            let name = req.params.as_ref().and_then(|p| p.get("name")).and_then(|v| v.as_str()).unwrap_or("");
            if app.names.contains(&name.to_string()) {
                let parameter_id = app.interface.get_parameter_id_from_name(name.to_owned());
                match parameter_id {
                    Some(parameter_id) => {
                        let value = app.interface.get(parameter_id, false);
                        match value {
                            Ok(value) => serde_json::json!({"pm": {name: value}}),
                            Err(_) => reply_error("Internal error"),
                        }
                    },
                    _ => reply_error("Internal error"),
                }
            }
            else {
                reply_error("Unknown parameter")
            }
        }
        "write" => {
            if let Some(params) = req.params.as_ref() {
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                if app.names.contains(&name.to_string()) {
                    let parameter_id = app.interface.get_parameter_id_from_name(name.to_owned());
                    match parameter_id {
                        Some(parameter_id) => {
                            // let parameter= ;
                            // let value = app.interface.set(parameter_id, parameter);
                            // match value {
                            //     Ok(value) => serde_json::json!({"pm": {name: value}}),
                            //     Err(_) => reply_error("Internal error"),
                            // }
                        },
                        _ => reply_error("Internal error"),
                    }
                }
                else {
                    reply_error("Unknown parameter")
                }
                // if name == "int_param" {
                //     if let Some(value) = params.get("value").and_then(|v| v.as_i64()) {
                //         app.int_param = value as i32;

                //         let notification = serde_json::json!({
                //             "jsonrpc": "2.0",
                //             "method": "notify",
                //             "params": {
                //                 "name": "int_param",
                //                 "value": app.int_param
                //             }
                //         })
                //         .to_string();

                //         app.subscribers.retain(|tx| tx.send(notification.clone()).is_ok());
                //     }
                // }
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
