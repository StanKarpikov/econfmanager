use econfmanager::interface::InterfaceInstance;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use warp::ws::Message;

#[derive(Default)]
pub(crate) struct AppState {
    pub subscribers: Vec<Vec<mpsc::UnboundedSender<Message>>>,
    pub interface: InterfaceInstance,
    pub names: Vec<String>,
}

pub(crate) type SharedState = Arc<Mutex<AppState>>;
