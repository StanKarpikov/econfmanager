/******************************************************************************
 * CONSTANTS
 ******************************************************************************/

const WEBSOCKET_ADDRESS = "ws://localhost:3031/api_ws";
const REQUEST_TIMEOUT = 5000;
const MAX_RECONNECT_ATTEMPTS = 10;
const RECONNECT_INTERVAL = 5000;

/******************************************************************************
 * VARIABLES
 ******************************************************************************/

let ws = null;

let reconnectTimer = null;
let reconnectAttempts = 0;

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
            reject(new Error(`Request ${request.id} timed out after ${REQUEST_TIMEOUT}ms`));
            cleanupRequest(request.id);
        }, REQUEST_TIMEOUT);

        pendingRequests[request.id] = {
            resolve: (result) => {
                resolve(result);
                cleanupRequest(request.id);
            },
            reject: (error) => {
                reject(error);
                cleanupRequest(request.id);
            },
            timeout: timeoutId
        };

        if (ws.readyState !== WebSocket.OPEN) {
            cleanupRequest(request.id);
            reject(new Error('WebSocket is not connected'));
            return;
        }

        ws.send(JSON.stringify(request));
    });
}

/******************************************************************************
 * UI FUNCTIONS
 ******************************************************************************/

function updateParam(id, value) {
    try {
        if (!parameters[id]) {
            console.error(`Parameter ${id} not found`);
            return;
        }

        parameters[id].value = value;

        const element = parameters[id].element;
        if (element) {
            element.removeEventListener("change", parameters[id].changeHandler);
            
            if (element.type === 'checkbox') {
                element.checked = Boolean(value);
            } else if (element.type === 'number' || element.type === 'range') {
                element.value = Number(value);
            } else if (element.type === 'text' || element.tagName === 'TEXTAREA') {
                element.value = String(value);
            } else if (element.tagName === 'SELECT') {
                element.value = String(value);
            } else {
                element.value = value;
            }

            element.addEventListener("change", parameters[id].changeHandler);
        }
    } catch (error) {
        console.error(`Error updating parameter ${id}:`, error);
    }
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
        const changeHandler = async () => {
            await writeParameter(id);
        };

        parameters[id] = {
            id,
            group,
            element: document.getElementById(id),
            changeHandler
        };

        if (parameters[id].element) {
            if (parameters[id].element) {
                parameters[id].element.addEventListener("change", parameters[id].changeHandler);
            }
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
        
        const result = await sendRpc("write", { 
            name: requestName, 
            value: value 
        });
        if (result && result.pm && result.pm[requestName] !== undefined) {
            updateParam(id, result.pm[requestName]);
            console.log(`Successfully updated ${id}`);
        }
        else
        {
            console.log(`Error updating paramter. Got result: ${JSON.stringify(result, null, 4)}`);
        }
        
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
    if (msg.params) {
        Object.keys(msg.params).forEach(function(key) {
            try {
                const [group, id] = key.split('@');
                if (parameters[id] && parameters[id].group === group) {
                    updateParam(id, msg.params[key]);
                }
            } catch (e) {
                console.error(`Error processing parameter ${key}: ${e}`);
            }
        });
    }
    else
    {
        console.warning("Could not get the parameters list, msg.params=", msg.params);
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

function handleWebSocketMessage(event) {
    try {
        console.info("Got message:", event);
        const msg = JSON.parse(event.data);
        
        if (msg.id && pendingRequests[msg.id]) {
            const request = pendingRequests[msg.id];
            if (msg.error) {
                request.reject(msg.error);
            } else {
                request.resolve(msg.result);
            }
            cleanupRequest(msg.id);
        }
        else if (msg.method === "notify") {
            console.info("Notification");
            process_notification(msg);
        }
    } catch (e) {
        console.error("Error processing message:", e);
    }
};


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