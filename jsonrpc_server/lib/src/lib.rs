pub mod configfile;
pub mod rest_server;
pub mod shared_state;
pub mod utils;
pub mod ws_server;

use econfmanager::interface::InterfaceInstance;
use warp::Filter;

use crate::configfile::Config;
use crate::rest_server::{handle_info, handle_read_param, handle_write_param};
use crate::shared_state::AppState;
use crate::ws_server::handle_ws;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

const PERIODIC_UPDATE_INTERVAL: Duration = Duration::from_millis(5000);


pub fn build_default_routes(
    config_file: String,
) -> (
    impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone,
    impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone,
    impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone,
    impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone,
    SocketAddr,
) {
    let config = Config::from_file(config_file.to_owned());

    let mut interface_instance = InterfaceInstance::new(
        &config.database_path,
        &config.saved_database_path,
        &config.default_data_folder,
    )
    .unwrap();
    interface_instance.start_periodic_update(PERIODIC_UPDATE_INTERVAL);
    let parameter_names = interface_instance.get_parameter_names();

    let state = Arc::new(Mutex::new(AppState {
        subscribers: (0..interface_instance.get_parameters_number())
            .map(|_| Vec::new())
            .collect(),
        interface: interface_instance,
        names: parameter_names,
    }));

    let state_filter = warp::any().map(move || state.clone());

    // WebSocket route
    let ws = warp::path("api_ws")
        .and(warp::ws())
        .and(state_filter.clone())
        .map(|ws: warp::ws::Ws, state| ws.on_upgrade(move |socket| handle_ws(socket, state)));

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
        .and(warp::body::content_length_limit(1024 * 1024))
        .and(warp::body::bytes())
        .and(state_filter.clone())
        .and_then(handle_write_param);

    let addr_str = format!(
        "{}:{}",
        config.json_rpc_listen_address, config.json_rpc_port
    );
    let socket_addr: SocketAddr = addr_str
        .parse()
        .expect("Failed to parse json_rpc_listen_address and json_rpc_port");

    (ws, read_param, write_param, info, socket_addr)
}

#[macro_export]
macro_rules! build_server {
    ($config_file:expr, $serve_static:expr, $($user_routes:expr),+) => {{
        pub async fn __internal_run_server(config_file: String) {
            use $crate::{build_default_routes};
            use warp::Filter;
            use warp::Reply;
            use warp::Rejection;
            use warp::path::FullPath;

            let (ws, read_param, write_param, info, socket_addr) =
                build_default_routes(config_file);
            
            let api_routes = ws
                        .or(read_param)
                        .or(write_param)
                        .or(info);
            $(
                let api_routes = api_routes.or($user_routes);
            )*

            let log = warp::log::custom(|info| {
                println!(
                    "{} {} {} {}",
                    info.method(),
                    info.path(),
                    info.status(),
                    info.elapsed().as_millis()
                );
            });

            if $serve_static {
                let static_files_path = std::env::var("STATIC_FILES_PATH").expect(
                    "STATIC_FILES_PATH environment variable not set"
                );
                let static_files = warp::fs::dir(static_files_path.clone());
                let fallback = warp::get()
                    .and(warp::path::full())
                    .map(move |_| {
                        match std::fs::read_to_string(format!("{}/index.html", static_files_path)) {
                            Ok(contents) => warp::reply::html(contents),
                            Err(_) => warp::reply::html("Index file not found".to_owned())
                        }
                    });
                warp::serve(api_routes.or(static_files).or(fallback).with(log))
                    .run(socket_addr)
                    .await;
            } else {
                warp::serve(api_routes.with(log))
                    .run(socket_addr)
                    .await;
            }
        }
        __internal_run_server($config_file).await
    }};

    ($config_file:expr, $serve_static:expr) => {
        $crate::build_server!($config_file, $serve_static, warp::any().map(|| ""))
    };
}
