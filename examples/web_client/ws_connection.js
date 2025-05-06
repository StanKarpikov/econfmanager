
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
        jrpc_process(msg);
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
