const ws = new WebSocket("ws://localhost:3030/ws");

const REQUEST_TIMEOUT = 5000;

const pendingRequests = {};

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

function updateParam(name, value) {
    const element = document.getElementById(name);
    if (element) {
        element.textContent = value;
    }
    if (name === "int_param") {
        const input = document.getElementById("int_param_input");
        if (input) {
            input.value = value;
        }
    }
}

async function init() {
    try {
        const result = await sendRpc("read", { name: "int_param" });
        console.log("Success:", result);
        updateParam("int_param", result.pm.int_param);
    } catch (error) {
        console.error("Error:", error);
    }
}

ws.onopen = () => {
    if (ws.readyState === WebSocket.OPEN) {
        init();
    }
};

const intParamInput = document.getElementById("int_param_input");
if (intParamInput) {
    intParamInput.addEventListener("change", e => {
        const value = parseInt(e.target.value);
        if (!isNaN(value)) {
            sendRpc("write", { name: "int_param", value }, res => {
                console.log("Write result:", res);
            });
        }
    });
}

ws.onmessage = event => {
    try {
        const msg = JSON.parse(event.data);
        
        if (msg.id && pendingRequests[msg.id]) {
            cleanupRequest(msg.id);
            if (msg.error) {
                pendingRequests[msg.id].reject(new Error(msg.error));
            } else {
                pendingRequests[msg.id].resolve(msg.result);
            }
        }
        else if (msg.method === "notify") {
            // Notification handling logic
        }
    } catch (e) {
        console.error("Error processing message:", e);
    }
};

ws.onerror = (error) => {
    console.error("WebSocket error:", error);
};

ws.onclose = () => {
    Object.keys(pendingRequests).forEach(id => {
        cleanupRequest(id);
    });
    console.log("WebSocket connection closed - cleaned up pending requests");
};