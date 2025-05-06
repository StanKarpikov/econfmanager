use arguments::Args;
use clap::Parser;
use configfile::Config;
use econfmanager::interface::{InterfaceInstance, ParameterUpdateCallback};
use econfmanager::generated::ParameterId;
use env_logger::Env;
use serde::{Deserialize, Serialize};
use warp::Rejection;
use std::{net::SocketAddr, sync::{Arc, Mutex}};
use warp::{Filter, ws::{Message, WebSocket}};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use log::{debug, error, info, warn};
use std::io::Write;
use warp::{http::StatusCode, reply::json};
use serde_json::json;

pub mod arguments;
pub mod configfile;

const SERVE_STATIC_FILES: bool = true;

#[derive(Clone)]
struct RouteInfo {
    path: String,
    method: String,
    description: String,
}

lazy_static::lazy_static! {
    static ref ROUTES: Vec<RouteInfo> = vec![
        RouteInfo {
            path: "/ws".to_string(),
            method: "GET".to_string(),
            description: "WebSocket connection endpoint".to_string(),
        },
        RouteInfo {
            path: "/api/read/:parameter".to_string(),
            method: "GET".to_string(),
            description: "Read a parameter value".to_string(),
        },
        RouteInfo {
            path: "/api/write/:parameter".to_string(),
            method: "POST".to_string(),
            description: "Write a parameter value".to_string(),
        },
        RouteInfo {
            path: "/info".to_string(),
            method: "GET".to_string(),
            description: "Shown info about the API".to_string(),
        },
    ];
}

#[derive(Default)]
struct AppState {
    subscribers: Vec<Vec<mpsc::UnboundedSender<Message>>>,
    interface: InterfaceInstance,
    names: Vec<String>,
}

type SharedState = Arc<Mutex<AppState>>;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info"))
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
    .init();

    let args = Args::parse();
    let config = Config::from_file(args.config);

    let interface_instance = InterfaceInstance::new(&config.database_path, &config.saved_database_path, &config.default_data_folder).unwrap();
    let parameter_names = interface_instance.get_parameter_names();
    
    let state = Arc::new(Mutex::new(AppState {
        subscribers: (0..interface_instance.get_parameters_number()).map(|_| Vec::new()).collect(),
        interface: interface_instance,
        names: parameter_names,
    }));

    let state_filter = warp::any().map(move || state.clone());

    // WebSocket route
    let ws = warp::path("api_ws")
        .and(warp::ws())
        .and(state_filter.clone())
        .map(|ws: warp::ws::Ws, state| {
            ws.on_upgrade(move |socket| handle_ws(socket, state))
        });

    // REST API routes
    let read_param = warp::path!("api" / "read" / String)
        .and(warp::get())
        .and(state_filter.clone())
        .and_then(handle_read_param);

    let info = warp::path!("api" / "info")
        .and(warp::get())
        .and(state_filter.clone())
        .and_then(handle_info);

    let write_param = warp::path!("api" / "write" / String)
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 1024)) // 1M max
        .and(warp::body::bytes())
        .and(state_filter.clone())
        .and_then(handle_write_param);

    let addr_str = format!("{}:{}", config.json_rpc_listen_address, config.json_rpc_port);
    let socket_addr: SocketAddr = addr_str
        .parse()
        .expect("Failed to parse json_rpc_listen_address and json_rpc_port");

    info!("Listening on http://{}", socket_addr);

    let api_routes = ws.or(read_param).or(write_param).or(info);
    if SERVE_STATIC_FILES {
        let static_files = warp::fs::dir("../examples/web_client");        
        let api_routes = api_routes.or(static_files);
        warp::serve(api_routes).run(socket_addr).await;
    }
    else {
        warp::serve(api_routes).run(socket_addr).await;
    }
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
    let parameter_name = app.interface.get_name(id);

    let Ok(value) = app.interface.get(id, false) else {
        let op = app.interface.get(id, false).unwrap_err();
        error!("Could not read parameter {} in notification: {}", id as usize, op);
        return;
    };

    let notification = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notify",
        "params": {
            parameter_name.clone(): InterfaceInstance::value_to_string(&value),
        }
    })
    .to_string();

    debug!("Notify subscribers for ID {} {}: {}", id as usize, parameter_name, notification);
    for tx in app.subscribers[id as usize].clone() {
        match tx.send(Message::text(notification.clone())) {
            Ok(_) => {},
            Err(err) => {
                error!("Failed notification: {}", err);
            },
        }
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
            debug!("Got read request {:?}", req.params);
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
            debug!("Got write request {:?}", req.params);
            let params = req.params.as_ref().ok_or_else(|| {
                let msg = "Missing parameters";
                error!("{}", msg);
                msg
            })?;
            
            let name = params.get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    let msg = "Could not decode parameter name";
                    error!("{}", msg);
                    msg
                })?;

            if !app.names.contains(&name.to_string()) {
                let msg = format!("Unknown parameter {}", name);
                error!("{}", msg);
                return Err(msg);
            }

            let parameter_id = app.interface.get_parameter_id_from_name(name.to_string())
                .ok_or_else(|| {
                    let msg = format!("Could not find parameter ID for {}", name);
                    error!("{}", msg);
                    msg
                })?;

            let value = params.get("value")
                .ok_or_else(|| {
                    let msg = "Missing value field";
                    error!("{}", msg);
                    msg
                })?;

            let value_string = match value {
                serde_json::Value::Null => value.to_string(),
                serde_json::Value::Bool(_) => value.to_string(),
                serde_json::Value::Number(_) => value.to_string(),
                serde_json::Value::String(_) => value.as_str().unwrap().to_owned(),
                serde_json::Value::Array(_) => value.to_string(),
                serde_json::Value::Object(_) => value.to_string(),
            };
            let converted = app.interface.set_from_string(parameter_id, &value_string)
                .map_err(|e| {
                    let max_len = 32;
                    let truncated_value: String = value_string.chars().take(max_len).collect();
                    let msg = format!("Unsupported type of |{}| id {} {}: {}", truncated_value, parameter_id as usize, name, e);
                    error!("{}", msg);
                    msg
                })?;

            let applied = app.interface.set(parameter_id, converted)
                .map_err(|e| format!("Failed to set the parameter {} id {} {}", e, parameter_id as usize, name))?;

            Ok(serde_json::json!({ "pm": { name: applied } }))
        },

        "save" => {
            debug!("Got save request");
            app.interface.save()
                .map_err(|e| format!("Could not save: {}", e))?;
            Ok(serde_json::json!({ "status": "saved" }))
        },

        "restore" => {
            debug!("Got restore request");
            app.interface.load()
                .map_err(|e| format!("Could not restore: {}", e))?;
            Ok(serde_json::json!({ "status": "restored" }))
        }

        "factory_reset" => {
            debug!("Got factory reset request");
            app.interface.factory_reset()
                .map_err(|e| format!("Could not do a factory reset: {}", e))?;
            Ok(serde_json::json!({ "status": "reset done" }))
        },

        _ => Err("Unknown method".into()),
    }
}

async fn handle_ws(ws: WebSocket, state: SharedState) {
    let (mut client_ws_tx, mut client_ws_rx) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));

    info!("Client connected");

    let mut forward_task = tokio::task::spawn(async move {
        while let Some(msg) = rx.recv().await {
            debug!("Send message {:?}", msg);
            if client_ws_tx.send(msg).await.is_err() {
                break; // Exit if send fails (connection closed)
            }
        }
    });

    let mut connection_active = true;
    
    while connection_active {
        tokio::select! {
            msg = client_ws_rx.next() => {
                debug!("Received message {:?}", msg);
                match msg {
                    Some(Ok(msg)) => {
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
                    Some(Err(e)) => {
                        info!("WebSocket error: {}", e);
                        connection_active = false;
                    },
                    None => {
                        info!("Client disconnected gracefully");
                        connection_active = false;
                    }
                }
            },

            _ = interval.tick() => {
                if tx.send(Message::ping(vec![])).is_err() {
                    connection_active = false;
                }
            },

            _ = &mut forward_task => {
                info!("Forwarding task terminated");
                connection_active = false;
            }
        }
    }

    let mut app = match state.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            warn!("Mutex poisoned, attempting recovery");
            poisoned.into_inner()
        }
    };

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
}

#[derive(Debug, Serialize)]
struct ParameterInfo {
    id: usize,
    name: String,
    comment: String,
    title: String,
    is_const: bool,
    runtime: bool,
    group: String,
    parameter_type: String,
}

#[derive(Debug, Serialize)]
struct GroupInfo {
    comment: String,
    title: String,
    name: String,
}

async fn handle_info(state: SharedState) -> Result<impl warp::Reply, warp::Rejection> {
    let app = state.lock().unwrap();
    let routes_json = ROUTES.iter().map(|r| {
        json!({
            "path": r.path,
            "method": r.method,
            "description": r.description
        })
    }).collect::<Vec<_>>();

    let parameters: Vec<ParameterInfo> = app.names.iter()
        .enumerate()
        .map(|(idx, _)| {
            let id = ParameterId::try_from(idx).unwrap();
            ParameterInfo {
                id: id as usize,
                name: app.interface.get_name(id),
                comment: app.interface.get_comment(id),
                title: app.interface.get_title(id),
                parameter_type: app.interface.get_type_string(id),
                is_const: app.interface.get_is_const(id),
                runtime: app.interface.get_runtime(id),
                group: app.interface.get_group(id),
            }
        })
        .collect();

    let group_parameters = app.interface.get_groups();
    let groups: Vec<GroupInfo> = group_parameters.iter()
        .map(|(name, title, comment)| {
            GroupInfo {
                name: name.to_string(),
                comment: comment.to_string(),
                title: title.to_string(),
            }
        })
        .collect();

    Ok(warp::reply::with_status(
        json(&json!({"parameters": parameters, "group": groups, "routes": routes_json})),
        StatusCode::OK,
    ))
}

async fn handle_read_param(name: String, state: SharedState) -> Result<impl warp::Reply, warp::Rejection> {
    let app = state.lock().unwrap();
    
    if !app.names.contains(&name) {
        let error_response = json(&json!({
            "error": format!("Parameter |{}| does not exist", name)
        }));
        return Ok(warp::reply::with_status(
            error_response,
            StatusCode::NOT_FOUND,
        ));
    }

    let parameter_id = match app.interface.get_parameter_id_from_name(name.clone()) {
        Some(id) => id,
        None => {
            let error_response = json(&json!({
                "error": format!("Could not find ID for parameter |{}|", name)
            }));
            return Ok(warp::reply::with_status(
                error_response,
                StatusCode::NOT_FOUND,
            ));
        }
    };

    match app.interface.get(parameter_id, false) {
        Ok(value) => Ok(warp::reply::with_status(
            json(&json!(value)),
            StatusCode::OK,
        )),
        Err(err) => {
            let error_response = json(&json!({
                "error": format!("Failed to read parameter |{}|: {:?}", name, err)
            }));
            Ok(warp::reply::with_status(
                error_response,
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn handle_write_param(
    name: String,
    value_bytes: warp::hyper::body::Bytes,
    state: SharedState,
) -> Result<impl warp::Reply, Rejection> {
    let value_str = match String::from_utf8(value_bytes.to_vec()) {
        Ok(s) => s,
        Err(e) => {
            let error_response = json(&json!({
                "error": format!("Invalid UTF-8 data: {}", e)
            }));
            return Ok(warp::reply::with_status(
                error_response,
                StatusCode::BAD_REQUEST,
            ));
        }
    };

    let app = state.lock().unwrap();
    
    if !app.names.contains(&name) {
        let error_response = json(&json!({
            "error": format!("Parameter |{}| does not exist", name)
        }));
        return Ok(warp::reply::with_status(
            error_response,
            StatusCode::NOT_FOUND,
        ));
    }

    let parameter_id = match app.interface.get_parameter_id_from_name(name.clone()) {
        Some(id) => id,
        None => {
            let error_response = json(&json!({
                "error": format!("No ID found for parameter |{}|", name)
            }));
            return Ok(warp::reply::with_status(
                error_response,
                StatusCode::NOT_FOUND,
            ));
        }
    };

    let converted = match app.interface.set_from_string(parameter_id, &value_str) {
        Ok(v) => v,
        Err(e) => {
            let error_response = json(&json!({
                "error": format!("Invalid parameter |{}| value |{}|: {}", name, value_str, e)
            }));
            return Ok(warp::reply::with_status(
                error_response,
                StatusCode::BAD_REQUEST,
            ));
        }
    };

    match app.interface.set(parameter_id, converted) {
        Ok(applied) => {
            let success_response = json(&json!(
                applied
            ));
            Ok(warp::reply::with_status(
                success_response,
                StatusCode::OK,
            ))
        },
        Err(e) => {
            let error_response = json(&json!({
                "error": format!("Failed to set parameter |{}|: {}", name, e)
            }));
            Ok(warp::reply::with_status(
                error_response,
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}
