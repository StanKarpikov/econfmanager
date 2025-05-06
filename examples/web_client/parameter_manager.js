/******************************************************************************
 * PARAMETERS FUNCTIONS
 ******************************************************************************/

async function fetchParameters() {
    try {
        const response = await fetch(`http://${SERVER_ADDRESS}/api/info`);
        if (!response.ok) {
            throw new Error('Failed to fetch parameters');
        }
        return await response.json();
    } catch (error) {
        console.error('Error fetching parameters:', error);
        return null;
    }
}

async function readParameter(param) {
    try {
        const requestName = param.name;
        const result = await sendRpc("read", { name: requestName });
        
        if (result && result.pm && result.pm[requestName] !== undefined) {
            updateParam(param.name, result.pm[requestName]);
        }
    } catch (error) {
        console.error(`Error reading parameter ${param.name}:`, error);
    }
}

async function writeParameter(param) {
    try {
        ui_get_param(param).then(async value => {
            const requestName = param.name;
            
            const result = await sendRpc("write", { 
                name: requestName, 
                value: value 
            });
            if (result && result.pm && result.pm[requestName] !== undefined) {
                updateParam(param.name, result.pm[requestName]);
                console.log(`Successfully updated ${param.name}`);
            }
            else
            {
                console.log(`Error updating paramter. Got result: ${JSON.stringify(result, null, 4)}`);
            }
        });
        
    } catch (error) {
        console.error(`Error updating parameter ${param.name}:`, error);
    }
}

async function readAllParameters() {
    if (ws && ws.readyState === WebSocket.OPEN) {
        console.log("Reading all parameters...");
        parameters.forEach(async param => {
            await readParameter(param);
        });
    }
}

async function parameters_save() {
    await sendRpc("save", {});
}

async function parameters_restore() {
    await sendRpc("restore", {});
}

async function parameters_factory_reset() {
    await sendRpc("factory_reset", {});
}