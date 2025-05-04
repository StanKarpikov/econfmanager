const ws = new WebSocket("ws://localhost:3030/ws");

function sendRpc(method, params, callback) {
    const request = {
        jsonrpc: "2.0",
        id: Date.now(),
        method,
        params
    };

    fetch(rpcUrl, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(request)
    })
    .then(res => res.json())
    .then(res => callback(res.result));
}

function updateParam(name, value) {
    document.getElementById(name).textContent = value;
    if (name === "int_param") {
        document.getElementById("int_param_input").value = value;
    }
}

function init() {
    sendRpc("read", { name: "string_param" }, res => updateParam("string_param", res.value));
    sendRpc("read", { name: "int_param" }, res => updateParam("int_param", res.value));
}

document.getElementById("int_param_input").addEventListener("change", e => {
    const value = parseInt(e.target.value);
    sendRpc("write", { name: "int_param", value }, res => {
        console.log("Write result:", res);
    });
});

ws.onmessage = event => {
    const msg = JSON.parse(event.data);
    if (msg.method === "notify") {
        updateParam(msg.params.name, msg.params.value);
    }
};

init();