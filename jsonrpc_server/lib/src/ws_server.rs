use crate::shared_state::{AppState, SharedState};
use econfmanager::interface::{InterfaceInstance, ParameterUpdateCallback};
use econfmanager::generated::ParameterId;
use serde::{Deserialize, Serialize};
use std::{sync::{Arc, Mutex}};
use warp::{ws::{Message, WebSocket}};
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use log::{debug, error, info, warn};
use crate::utils::debug_limited;

#[derive(Deserialize)]
pub(crate) struct RpcRequest {
    id: serde_json::Value,
    method: String,
    params: Option<serde_json::Value>,
}

#[derive(Serialize)]
pub struct RpcResponse {
    id: serde_json::Value,
    result: serde_json::Value,
}

pub(crate) fn handle_rpc_logic_ws(
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

            if app.interface.is_internal(parameter_id)
            {
                let msg = format!("Access internal parameter |{}| forbidden", name);
                error!("{}", msg);
                return Err(msg);
            }

            let value = app.interface.get(parameter_id, false)
                .map_err(|e| format!("Internal error: {}", e))?;

            if app.subscribers[parameter_id as usize].is_empty() {
                let state: Arc<Mutex<_>> = Arc::clone(&state);
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
            debug_limited(&format!("Got write request {:?}", req.params), 100);
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

            if app.interface.is_internal(parameter_id)
            {
                let msg = format!("Access internal parameter |{}| forbidden", name);
                error!("{}", msg);
                return Err(msg);
            }

            if app.interface.is_readonly(parameter_id)
            {
                let msg = format!("Readonly parameter cannnot be changed |{}|", name);
                error!("{}", msg);
                return Err(msg);
            }
            
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

pub(crate) fn notify_client(app: &mut AppState, id: ParameterId) {
    if app.interface.is_internal(id)
    {
        return;
    }

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

    debug_limited(&format!("Notify subscribers for ID {} {}: {}", id as usize, parameter_name, notification), 100);
    for tx in app.subscribers[id as usize].clone() {
        match tx.send(Message::text(notification.clone())) {
            Ok(_) => {},
            Err(err) => {
                error!("Failed notification: {}", err);
            },
        }
    }
}

pub(crate) async fn handle_ws(ws: WebSocket, state: SharedState) {
    let (mut client_ws_tx, mut client_ws_rx) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));

    info!("Client connected");

    let mut forward_task = tokio::task::spawn(async move {
        while let Some(msg) = rx.recv().await {
            debug_limited(&format!("Send message {:?}", msg), 100);
            if client_ws_tx.send(msg).await.is_err() {
                break; // Exit if send fails (connection closed)
            }
        }
    });

    let mut connection_active = true;
    
    while connection_active {
        tokio::select! {
            msg = client_ws_rx.next() => {
                debug_limited(&format!("Received message {:?}", msg), 100);
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