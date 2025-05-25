use econfmanager::generated::ParameterId;
use serde::Serialize;
use warp::Rejection;
use warp::{http::StatusCode, reply::json};
use serde_json::json;

use crate::shared_state::SharedState;

// use crate::SharedState;

#[derive(Clone)]
struct RouteInfo {
    path: String,
    method: String,
    description: String,
}

#[derive(Debug, Serialize)]
struct ParameterInfo {
    id: usize,
    name: String,
    comment: String,
    title: String,
    is_const: bool,
    runtime: bool,
    readonly: bool,
    group: String,
    tags: Vec<String>,
    validation: serde_json::Value,
    parameter_type: String,
}

#[derive(Debug, Serialize)]
struct GroupInfo {
    comment: String,
    title: String,
    name: String,
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

pub(crate) async fn handle_info(state: SharedState) -> Result<impl warp::Reply, warp::Rejection> {
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
        .filter(|(idx, _)| {
            let id = ParameterId::try_from(*idx).unwrap();
            !app.interface.is_internal(id)
        })
        .map(|(idx, _)| {
            let id = ParameterId::try_from(idx).unwrap();
            ParameterInfo {
                id: id as usize,
                name: app.interface.get_name(id),
                comment: app.interface.get_comment(id),
                title: app.interface.get_title(id),
                parameter_type: app.interface.get_type_string(id),
                is_const: app.interface.is_const(id),
                runtime: app.interface.is_runtime(id),
                validation: app.interface.get_validation_json(id),
                group: app.interface.get_group(id),
                readonly: app.interface.is_readonly(id),
                tags: app.interface.get_tags(id),
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

pub(crate) async fn handle_read_param(name: String, state: SharedState) -> Result<impl warp::Reply, warp::Rejection> {
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

    if app.interface.is_internal(parameter_id)
    {
        let error_response = json(&json!({
            "error": format!("Access internal parameter |{}| forbidden", name)
        }));
        return Ok(warp::reply::with_status(
            error_response,
            StatusCode::FORBIDDEN,
        ));
    }

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

pub(crate) async fn handle_write_param(
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

    if app.interface.is_internal(parameter_id)
    {
        let error_response = json(&json!({
            "error": format!("Access internal parameter |{}| forbidden", name)
        }));
        return Ok(warp::reply::with_status(
            error_response,
            StatusCode::FORBIDDEN,
        ));
    }

    if app.interface.is_readonly(parameter_id)
    {
        let error_response = json(&json!({
            "error": format!("Readonly parameter cannnot be changed |{}|", name)
        }));
        return Ok(warp::reply::with_status(
            error_response,
            StatusCode::FORBIDDEN,
        ));
    }

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
