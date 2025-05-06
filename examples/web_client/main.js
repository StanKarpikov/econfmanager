/******************************************************************************
 * CONSTANTS
 ******************************************************************************/

const SERVER_ADDRESS = "localhost:3031";
// const SERVER_ADDRESS = location.host;
const WEBSOCKET_ADDRESS = `ws://${SERVER_ADDRESS}/api_ws`;
const REQUEST_TIMEOUT = 5000;
const MAX_RECONNECT_ATTEMPTS = 10;
const RECONNECT_INTERVAL = 5000;

/******************************************************************************
 * VARIABLES
 ******************************************************************************/

let id_index = 1;

let ws = null;

let reconnectTimer = null;
let reconnectAttempts = 0;

const pendingRequests = {};

let parameters = {};

/******************************************************************************
 * EVENT LISTENERS
 ******************************************************************************/

document.addEventListener("DOMContentLoaded", () => {
    setupParameters();
    connectWebSocket();
});

window.addEventListener("beforeunload", () => {
    if (ws) {
        // Use status code 1000 (normal closure) to prevent automatic reconnection
        ws.close(1000, "Page unloading");
    }
});
