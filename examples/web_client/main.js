/******************************************************************************
 * CONSTANTS
 ******************************************************************************/

const WEBSOCKET_ADDRESS = "ws://localhost:3030";
const REQUEST_TIMEOUT = 5000;
const MAX_RECONNECT_ATTEMPTS = 10;
const RECONNECT_INTERVAL = 5000;

/******************************************************************************
 * VARIABLES
 ******************************************************************************/

let ws = null;

let reconnectTimer = null;

const pendingRequests = {};

const parameters = {};

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

/******************************************************************************
 * JSON-RPC FUNCTIONS
 ******************************************************************************/

function cleanupRequest(id) {
    if (pendingRequests[id]) {
        clearTimeout(pendingRequests[id].timeout);
        delete pendingRequests[id];
    }
}

function sendRpc(method, params) {
    return new Promise((resolve, reject) => {
        const request = {
            jsonrpc: "2.0",
            id: Date.now(),
            method,
            params
        };

        const timeoutId = setTimeout(() => {
            reject(new Error("Request timed out"));
            cleanupRequest(request.id);
        }, REQUEST_TIMEOUT);

        pendingRequests[request.id] = {
            resolve: callback,
            reject: (error) => {
                console.error("Request failed:", error);
                callback({ error: error.message });
            },
            timeout: timeoutId
        };
        ws.send(JSON.stringify(request));
    });
}

/******************************************************************************
 * PARAMETERS FUNCTIONS
 ******************************************************************************/

async function setupParameters() {
    setupParameter("image_width", "image_acquisition");
    setupParameter("image_height", "image_acquisition");
    setupParameter("exposure", "image_acquisition");
    setupParameter("device_name", "device");
    setupParameter("serial_number", "device");

    await readAllParameters();
}

function setupParameter(id, group) {
    try {
        parameters[id] = {
            id,
            group,
            element: document.getElementById(id)
        };

        if (parameters[id].element) {
            parameters[id].element.addEventListener("change", async () => {
                await writeParameter(id);
            });
        }
    } catch (error) {
        console.error(`Error setting up parameter ${id}:`, error);
    }
}

async function readParameter(id) {
    if (!parameters[id]) return;

    try {
        const requestName = `${parameters[id].group}@${id}`;
        const result = await sendRpc("read", { name: requestName });
        
        if (result && result.pm && result.pm[requestName] !== undefined) {
            updateParam(id, result.pm[requestName]);
        }
    } catch (error) {
        console.error(`Error reading parameter ${id}:`, error);
    }
}

async function writeParameter(id) {
    if (!parameters[id] || !parameters[id].element) return;

    try {
        const value = parameters[id].element.value;
        const requestName = `${parameters[id].group}@${id}`;
        
        await sendRpc("write", { 
            name: requestName, 
            value: value 
        });
        
        console.log(`Successfully updated ${id}`);
    } catch (error) {
        console.error(`Error updating parameter ${id}:`, error);
    }
}

async function readAllParameters() {
    if (ws && ws.readyState === WebSocket.OPEN) {
        console.log("Reading all parameters...");
        for (const id in parameters) {
            await readParameter(id);
        }
    }
}

function process_notification(msg)
{
    const paramFullName = msg.params.parameter;
    if (paramFullName) {
        const [group, id] = paramFullName.split('@');
        if (parameters[id] && parameters[id].group === group) {
            updateParam(id, msg.params.value);
        }
    }
}

/******************************************************************************
 * WEBSOCKET MANAGEMENTS
 ******************************************************************************/

function connectWebSocket() {
    if (reconnectTimer) {
        clearTimeout(reconnectTimer);
        reconnectTimer = null;
    }

    if (ws) {
        ws.close();
    }

    ws = new WebSocket(WEBSOCKET_ADDRESS);

    ws.onopen = handleWebSocketOpen;
    ws.onclose = handleWebSocketClose;
    ws.onmessage = handleWebSocketMessage;
    ws.onerror = handleWebSocketError;
}

function handleWebSocketOpen() {
    console.log("WebSocket connected");
    reconnectAttempts = 0;
    window.connectionUI.updateConnectionStatus('connected');
    window.connectionUI.logConnectionEvent('Connection established');
    window.connectionUI.updateAttemptCount(0, MAX_RECONNECT_ATTEMPTS);
    
    readAllParameters().catch(error => {
        console.error("Error reading parameters after connection:", error);
    });
}

function handleWebSocketClose(event) {
    console.log("WebSocket disconnected", event.code, event.reason);
    window.connectionUI.updateConnectionStatus('disconnected');
    window.connectionUI.logConnectionEvent(`Disconnected (code ${event.code}: ${event.reason || 'no reason'})`);
    
    if (event.code !== 1000) {
        attemptReconnect();
    }
}

function handleWebSocketError(error) {
    console.error("WebSocket error:", error);
    window.connectionUI.logConnectionEvent(`Error: ${error.message || 'Unknown error'}`);
}

function attemptReconnect() {
    if (reconnectAttempts >= MAX_RECONNECT_ATTEMPTS) {
        window.connectionUI.logConnectionEvent('Max reconnection attempts reached');
        return;
    }

    reconnectAttempts++;
    window.connectionUI.updateConnectionStatus('connecting');
    window.connectionUI.logConnectionEvent(`Reconnecting (attempt ${reconnectAttempts}/${MAX_RECONNECT_ATTEMPTS})...`);
    window.connectionUI.updateAttemptCount(reconnectAttempts, MAX_RECONNECT_ATTEMPTS);
    
    reconnectTimer = setTimeout(() => {
        connectWebSocket();
    }, RECONNECT_INTERVAL);
}