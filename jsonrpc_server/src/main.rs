use arguments::Args;
use clap::Parser;
use configfile::Config;
use econfmanager::interface::{InterfaceInstance, ParameterUpdateCallback};
use econfmanager::generated::ParameterId;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::{Arc, Mutex}};
use warp::{Filter, ws::{Message, WebSocket}};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use log::LevelFilter;
use log::info;
use std::io::Write;

pub mod arguments;
pub mod configfile;


#[derive(Default)]
struct AppState {
    subscribers: Vec<Vec<mpsc::UnboundedSender<Message>>>,
    interface: InterfaceInstance,
    names: Vec<String>,
}

type SharedState = Arc<Mutex<AppState>>;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] {}:{} - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .filter_level(LevelFilter::Debug)
        .init();

    let args = Args::parse();
    let config = Config::from_file(args.config);

    let interface_instance = InterfaceInstance::new(&config.database_path, &config.saved_database_path).unwrap();
    let parameter_names = interface_instance.get_parameter_names();
    
    let state = Arc::new(Mutex::new(AppState {
        subscribers: (0..interface_instance.get_parameters_number()).map(|_| Vec::new()).collect(),
        interface: interface_instance,
        names: parameter_names,
    }));

    let state_filter = warp::any().map(move || state.clone());

    let ws = warp::path("ws")
        .and(warp::ws())
        .and(state_filter.clone())
        .map(|ws: warp::ws::Ws, state| {
            ws.on_upgrade(move |socket| handle_ws(socket, state))
        });

    let addr_str = format!("{}:{}", config.json_rpc_listen_address, config.json_rpc_port);
    let socket_addr: SocketAddr = addr_str
        .parse()
        .expect("Failed to parse json_rpc_listen_address and json_rpc_port");

    println!("Listening on http://{}", socket_addr);
    warp::serve(ws).run(socket_addr).await;
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

fn notify_client(app: &mut AppState, id: ParameterId) {
    let notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notify",
        "params": {
            "id": id as usize
        }
    })
    .to_string();

    for tx in app.subscribers[id as usize].clone() {
        let _ = tx.send(Message::text(notification.clone()));
    }
}

fn handle_rpc_logic_ws(
    state: SharedState,
    req: &RpcRequest,
    client_tx: tokio::sync::mpsc::UnboundedSender<Message>,
) -> Result<serde_json::Value, String> {
    let mut app = state.lock().unwrap();

    match req.method.as_str() {
        "read" => {
            let name = req.params
                .as_ref()
                .and_then(|p| p.get("name"))
                .and_then(|v| v.as_str())
                .ok_or("Could not decode parameter name")?;

            if !app.names.contains(&name.to_string()) {
                return Err(format!("Unknown parameter {}", name));
            }

            let parameter_id = app.interface
                .get_parameter_id_from_name(name.to_string())
                .ok_or(format!("Could not find parameter ID for {}", name))?;

            let value = app.interface.get(parameter_id, false)
                .map_err(|e| format!("Internal error: {}", e))?;

            if app.subscribers[parameter_id as usize].is_empty() {
                let state = Arc::clone(&state);
                let callback = Arc::new(move |id: ParameterId| {
                    let state = Arc::clone(&state);
                    let mut app = state.lock().unwrap();
                    notify_client(&mut app, id);
                }) as ParameterUpdateCallback;

                app.interface.add_callback(parameter_id, callback)
                    .map_err(|e| format!("Internal error: {}", e))?;
            }

            // Subscribe this client if not already subscribed
            if !app.subscribers[parameter_id as usize]
                .iter()
                .any(|sub| sub.same_channel(&client_tx))
            {
                app.subscribers[parameter_id as usize].push(client_tx.clone());
            }

            Ok(serde_json::json!({ "pm": { name: value } }))
        }
        "write" => {
            let params = req.params.as_ref().ok_or("Missing parameters")?;
            let name = params.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| {return "Could not decode parameter name".into()});

            if !app.names.contains(&name.to_string()) {
                return Err(format!("Unknown parameter {}", name).into());
            }

            let parameter_id = app.interface.get_parameter_id_from_name(name.to_string())
                .ok_or(format!("Could not find parameter ID for {}", name))?;

            let value = params.get("value")
                .ok_or("Missing value field")?;

            let converted = app.interface.set_from_json(parameter_id, value)
                .map_err(|e| format!("Unsupported type {}", e))?;

            let applied = app.interface.set(parameter_id, converted)
                .map_err(|e| format!("Failed to set the parameter {}", e))?;

            Ok(serde_json::json!({ "pm": { name: applied } }))
        },

        "save" => {
            app.interface.save()
                .map_err(|e| format!("Could not save: {}", e))?;
            Ok(serde_json::json!({ "status": "saved" }))
        },

        "restore" => {
            app.interface.load()
                .map_err(|e| format!("Could not restore: {}", e))?;
            Ok(serde_json::json!({ "status": "restored" }))
        }

        _ => Err("Unknown method".into()),
    }
}

async fn handle_ws(ws: WebSocket, state: SharedState) {
    let (mut client_ws_tx, mut client_ws_rx) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    info!("Client connected");

    // Spawn a task to forward messages from internal channel to websocket
    tokio::task::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let _ = client_ws_tx.send(msg).await;
        }
    });

    while let Some(result) = client_ws_rx.next().await {
        match result {
            Ok(msg) => {
                if msg.is_text() {
                    if let Ok(req) = serde_json::from_str::<RpcRequest>(msg.to_str().unwrap()) {
                        let result = match handle_rpc_logic_ws(state.clone(), &req, tx.clone()) {
                            Ok(value) => value,
                            Err(error) => serde_json::json!({ "error": error }),
                        };
                        let response = RpcResponse {
                            id: req.id,
                            result,
                        };
                        let _ = tx.send(Message::text(serde_json::to_string(&response).unwrap()));
                    }
                }
            },
            Err(e) => {
                info!("Client disconnected ({})", e);

                let mut app = state.lock().unwrap();
                let mut indices_to_delete = Vec::new();

                for (idx, param_subscribers) in app.subscribers.iter_mut().enumerate() {
                    param_subscribers.retain(|sub| !sub.same_channel(&tx));
                    if param_subscribers.is_empty() {
                        indices_to_delete.push(idx);
                    }
                }

                for idx in indices_to_delete {
                    if let Ok(id) = ParameterId::try_from(idx) {
                        let _ = app.interface.delete_callback(id);
                    }
                }
                
                break; // Exit loop on error (e.g., connection closed)
            }
        }
    }
}
