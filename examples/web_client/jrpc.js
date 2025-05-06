
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
        console.info(`Send RPC`, params);
        const request = {
            jsonrpc: "2.0",
            id: id_index,
            method,
            params
        };
        id_index++;

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

function jrpc_process(msg) {
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
}

function process_notification(msg)
{
    if (msg.params) {
        Object.keys(msg.params).forEach(function(key) {
            try {
                updateParam(key, msg.params[key]);
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
