
/******************************************************************************
 * UI FUNCTIONS
 ******************************************************************************/

function updateParam(name, value) {
    try {
        const param = parameters.find(p => p.name === name || p.name.endsWith(`@${name}`));
        if (!param) {
            console.error(`Parameter ${name} not found`);
            return;
        }

        param.value = value;

        if (param.changeHandler) {
            param.element.removeEventListener("change", param.changeHandler);
        }

        switch(param.parameter_type.toLowerCase()) {
            case 'bool':
                param.element.checked = Boolean(value);
                break;
            case 'i32':
            case 'u32':
            case 'f32':
                param.element.value = Number(value);
                if (param.parameter_type.toLowerCase() === 'f32') {
                    param.element.value = parseFloat(value).toString();
                }
                break;
            case 'string':
                param.element.value = String(value);
                break;
            case 'blob':
                if (value) {
                    const blobInfo = param.element.querySelector('.blob-info');
                    const imagePreview = param.element.querySelector('.image-preview');
                    blobInfo.innerHTML = '';
                    imagePreview.innerHTML = '';
            
                    const sizeInBytes = Math.floor(value.length * 3 / 4);
                    blobInfo.textContent = `Size: ${formatFileSize(sizeInBytes)}`;
            
                    const img = document.createElement('img');
                    img.style.maxWidth = '200px';
                    img.style.maxHeight = '200px';
                    
                    img.onload = function() {
                        imagePreview.appendChild(img);
                    };
                    img.onerror = function() {
                        imagePreview.innerHTML = '';
                        const downloadBtn = document.createElement('button');
                        downloadBtn.textContent = 'Download File';
                        downloadBtn.onclick = () => {
                            const a = document.createElement('a');
                            a.href = `data:application/octet-stream;base64,${value}`;
                            a.download = 'file.bin';
                            a.click();
                        };
                        imagePreview.appendChild(downloadBtn);
                    };
                    
                    img.src = `data:image/png;base64,${value}`;
                }
                break;
            default:
                param.element.value = value;
        }

        if (param.changeHandler) {
            param.element.addEventListener("change", param.changeHandler);
        }

    } catch (error) {
        console.error(`Error updating parameter ${name}:`, error);
    }
}

function createParameterInput(param) {
    const paramGroup = document.createElement('div');
    paramGroup.className = 'param-group';
    
    let group_id = param.name.split('@')[0];
    let parameter_id = param.name.split('@')[1];

    const label = document.createElement('label');
    label.htmlFor = parameter_id;
    label.textContent = param.title + ':';
    
    let input;
    switch(param.parameter_type.toLowerCase()) {
        case 'bool':
            input = document.createElement('input');
            input.type = 'checkbox';
            input.id = parameter_id;
            break;
        case 'i32':
        case 'u32':
        case 'f32':
            input = document.createElement('input');
            input.type = 'number';
            input.id = parameter_id;
            if (param.parameter_type.toLowerCase() === 'f32') {
                input.step = 'any';
            }
            break;
        case 'string':
            input = document.createElement('input');
            input.type = 'text';
            input.id = parameter_id;
            break;
        case 'blob':
            input = document.createElement('div');
            input.className = 'blob-parameter';
            input.id = parameter_id;
            
            const fileInput = document.createElement('input');
            fileInput.type = 'file';
            fileInput.className = 'blob-input';
            
            const blobInfo = document.createElement('div');
            blobInfo.className = 'blob-info';
            
            const imagePreview = document.createElement('div');
            imagePreview.className = 'image-preview';
            
            input.appendChild(fileInput);
            input.appendChild(blobInfo);
            input.appendChild(imagePreview);
            break;
        default:
            input = document.createElement('input');
            input.type = 'text';
            input.id = parameter_id;
    }
    param.element = input;

    paramGroup.appendChild(label);
    paramGroup.appendChild(input);
    
    if (param.comment) {
        input.title = param.comment;
        
        const comment = document.createElement('div');
        comment.className = 'comment';
        comment.textContent = param.comment;
        paramGroup.appendChild(comment);
    }

    param.changeHandler = async () => {
        await writeParameter(param);
    };

    input.addEventListener("change", param.changeHandler);
    
    return paramGroup;
}

function createParameterSection(groupInfo, parameters) {
    const section = document.createElement('div');
    section.className = 'section';
    
    const heading = document.createElement('h2');
    heading.textContent = groupInfo.title;
    if (groupInfo.comment) {
        heading.title = groupInfo.comment;
    }
    section.appendChild(heading);
    
    const groupParams = parameters.filter(p => p.group === groupInfo.name);
    groupParams.forEach(param => {
        section.appendChild(createParameterInput(param));
    });
    
    return section;
}

async function setupParameters() {
    const config = await fetchParameters();
    if (!config) return;
    
    const mainContainer = document.getElementById('parameters') || document.body;
    
    parameters = config.parameters;

    config.group.forEach(groupInfo => {
        mainContainer.appendChild(createParameterSection(groupInfo, parameters));
    });
    
    await readAllParameters();
}

function formatFileSize(bytes) {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

async function fileToBase64(file) {
    let result_base64 = await new Promise((resolve, reject) => {
        const reader = new FileReader();
        reader.onload = e => resolve(e.target.result.split(',')[1] || e.target.result);
        reader.onerror = reject;
        reader.readAsDataURL(file);
    });
    return result_base64;
}

async function ui_get_param(param) {
    let value;
    switch(param.parameter_type.toLowerCase()) {
        case 'bool':
            value = param.element.checked ? "true" : "false";
            break;
        case 'i32':
        case 'u32':
            value = parseInt(param.element.value);
            break;
        case 'f32':
            value = parseFloat(param.element.value);
            break;
        case 'blob':
            value = "";
            if (param.element.querySelector('.blob-input').files.length > 0) {
                const fileInput = param.element.querySelector('.blob-input');
                if (fileInput.files.length > 0) {
                    const file = fileInput.files[0];
                    value = await fileToBase64(file);
                }
            }
            break;
        default:
            value = param.element.value;
    }
    return value;
}