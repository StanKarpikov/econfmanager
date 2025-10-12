#!/usr/bin/env python3
"""
Converter script to generate Python interface for the econfmanager C library.

This script parses the C header file and generates a Python script that creates
a ctypes-based interface to the C library.
"""

import re
import sys
from typing import List, Dict, Tuple

def parse_header(header_content: str) -> Tuple[Dict, List, List, List, Dict]:
    """
    Parse the C header file to extract enums, structs, typedefs, and function declarations.

    Args:
        header_content: Content of the header file

    Returns:
        Tuple containing:
        - Dictionary of enums
        - List of structs
        - List of function declarations
    """
    enums = {}
    structs = []
    functions = []

    # Parse enums
    enum_pattern = re.compile(
        r'typedef\s+enum\s*{\s*([^}]*)\s*}\s*(\w+)\s*;',
        re.DOTALL
    )

    for match in enum_pattern.finditer(header_content):
        enum_name = match.group(2)
        enum_values = {}

        for item in match.group(1).split(','):
            item = item.strip()
            if not item:
                continue

            # Handle both simple and complex enum items
            if '=' in item:
                name, value = item.split('=', 1)
                name = name.strip()
                value = value.strip().rstrip(',')
                enum_values[name] = value
            else:
                name = item.rstrip(',').strip()
                enum_values[name] = None

        enums[enum_name] = enum_values

    # Parse structs
    struct_pattern = re.compile(
        r'typedef\s+struct\s+(\w+)\s*{\s*([^}]*)\s*}\s*(\w+)\s*;',
        re.DOTALL
    )

    for match in struct_pattern.finditer(header_content):
        struct_name = match.group(3)
        struct_content = match.group(2).strip()
        structs.append((struct_name, struct_content))

    # Parse typedefs and build typedef_map
    typedef_pattern = re.compile(
        r'typedef\s+([^\s]+(?:\s*\*)?)\s+(\w+)\s*;'
    )

    typedefs = []
    typedef_map = {}
    for match in typedef_pattern.finditer(header_content):
        base_type = match.group(1).strip()
        alias = match.group(2).strip()
        typedefs.append(f"{base_type} {alias}")
        typedef_map[alias] = base_type

    # Parse function declarations
    func_pattern = re.compile(
        r'(\w+)\s+(\w+)\(([^)]*)\)\s*;'
    )

    for match in func_pattern.finditer(header_content):
        return_type = match.group(1)
        func_name = match.group(2)
        params = []

        # Extract parameters with their names and types
        for p in match.group(3).split(','):
            p = p.strip()
            if not p:
                continue

            # Extract parameter type and name
            param_parts = re.split('[ *]', p)
            print(f"Param parts: {param_parts}")

            # Handle const char* special case
            if len(param_parts) >= 4 and param_parts[0] == 'const' and param_parts[1] == 'char':
                param_type = 'const char*'
                param_name = param_parts[-1] if len(param_parts) > 2 else 'str_ptr'
            # Handle pointer types
            elif '*' in p:
                # Find the last space before the *
                last_space = p.rfind(' ', 0, p.find('*'))
                if last_space > 0:
                    param_name = p[last_space+1:].strip().replace('*', '')
                    param_type = p[:last_space].strip()
                else:
                    param_name = p.strip() if len(param_parts) > 1 else f"arg"
                    param_type = f"{param_type.replace('*', '')}_ptr"
            # Handle regular types
            else:
                param_type = param_parts[-2]
                param_name = param_parts[-1] if len(param_parts) > 1 else f"arg"

            print(f"Param: type `{param_type}` name `{param_name}`")

            # Store the parameter as a dictionary with type and name
            params.append({
                'type': param_type,
                'name': param_name,
                'full': p
            })

        functions.append({
            'return_type': return_type,
            'name': func_name,
            'params': params
        })

    return enums, structs, typedefs, functions, typedef_map

def generate_python_interface(enums: Dict, structs: List, typedefs: List, functions: List, typedef_map: Dict) -> str:
    """
    Generate Python interface code using ctypes.

    Args:
        enums: Dictionary of enums
        structs: List of structs
        typedefs: List of typedefs
        functions: List of function declarations
        typedef_map: Mapping of custom typedefs to base types

    Returns:
        Generated Python code as a string
    """
    python_code = """#!/usr/bin/env python3
\"\"\"
Python interface for the econfmanager C library.

Generated automatically - DO NOT EDIT
\"\"\"

from ctypes import *
from enum import IntEnum
from typing import Any
import ctypes.util
import os
import sys

class EconfManager:
    \"\"\"
    Python interface to the econfmanager C library.
    \"\"\"

    def __init__(self, lib_path=None):
        \"\"\"
        Initialize the econfmanager interface.

        Args:
            lib_path: Path to the econfmanager library. If None, tries to find it automatically.
        \"\"\"
        if lib_path is None:
            lib_path = os.path.join(os.path.dirname(__file__), 'libeconfmanager.so')

        self.lib = CDLL(lib_path)
        self._setup_types()
        self._setup_functions()
"""

    # Add enum definitions as Enum classes
    for enum_name, enum_values in enums.items():
        python_code += f"\n    # {enum_name} enum\n"
        python_code += f"    class {enum_name}(IntEnum):\n"

        for value_name, value in enum_values.items():
            if value is None:
                # For enums without explicit values, assign sequential integers
                index = list(enum_values.keys()).index(value_name)
                python_code += f"        {value_name} = {index}\n"
            else:
                python_code += f"        {value_name} = {value}\n"

    # Add struct definitions at class level
    for struct_name, struct_content in structs:
        python_code += f"\n    # {struct_name} struct\n"
        python_code += f"    class {struct_name}(Structure):\n"
        python_code += "        _fields_ = [\n"

        # Parse struct fields
        for line in struct_content.split('\n'):
            line = line.strip()
            if not line or line.startswith('//') or line.startswith('*'):
                continue

            # Handle field declarations
            if ';' in line:
                field_decl = line.rstrip(';').strip()
                if field_decl:
                    python_code += f"            # {field_decl}\n"

        python_code += "        ]\n"

    # Add typedefs at class level
    for typedef in typedefs:
        python_code += f"\n    # {typedef}\n"
        if 'POINTER' in typedef or 'Callback' in typedef or 'FFI' in typedef:
            # Handle pointer and callback types
            if 'ParameterUpdateCallbackFFI' in typedef:
                python_code += "    ParameterUpdateCallbackFFI = CFUNCTYPE(None, c_int32, c_void_p)\n"
            elif 'CInterfaceInstance' in typedef:
                python_code += "    CInterfaceInstancePtr = POINTER(CInterfaceInstance)\n"
                python_code += "    CInterfaceInstancePtrPtr = POINTER(CInterfaceInstancePtr)\n"

    # Generate _setup_types method
    python_code += """
    def _setup_types(self):
        \"\"\"
        Set up all the type definitions for the C library.
        \"\"\"
        # Types are already defined at class level
        pass
"""

    # Generate _setup_functions method
    python_code += """
    def _setup_functions(self):
        \"\"\"
        Set up all the function definitions for the C library.
        \"\"\"
"""

    # --- Type mapping for C types to ctypes ---
    ctype_map = {
        "void": "None",
        "void*": "c_void_p",
        "const void*": "c_void_p",
        "char": "c_char",
        "char*": "c_char_p",
        "const char*": "c_char_p",
        "int": "c_int",
        "int32_t": "c_int32",
        "int64_t": "c_int64",
        "uint32_t": "c_uint32",
        "uint64_t": "c_uint64",
        "uintptr_t": "c_size_t",
        "bool": "c_bool",
        "size_t": "c_size_t",
        "float": "c_float",
        "double": "c_double",
    }

    def resolve_ctype(c_type: str) -> str:
        t = c_type.strip()
        t = t.replace("const ", "")
        # Handle pointer types
        if t.endswith("**"):
            base = t[:-2].strip()
            resolved = resolve_ctype(typedef_map.get(base, base))
            return f"POINTER({resolved})"
        if t.endswith("*"):
            base = t[:-1].strip()
            # Special case for char* (string)
            if base == "char":
                return "c_char_p"
            resolved = resolve_ctype(typedef_map.get(base, base))
            return f"POINTER({resolved})"
        # Recursively resolve typedefs
        while t in typedef_map:
            t = typedef_map[t]
        # Special case for enums (use c_int)
        if t in enums:
            return "c_int"
        return ctype_map.get(t, f"c_void_p")

    def ctype_to_pytype(c_type: str) -> str:
        t = c_type.strip()
        t = t.replace("const ", "")
        if t.endswith("*"):
            base = t[:-1].strip()
            if base == "char":
                return "str"
            return "Any"
        # Recursively resolve typedefs
        while t in typedef_map:
            t = typedef_map[t]
        if t in enums:
            return "int"
        if t in ("int", "int32_t", "int64_t", "uint32_t", "uint64_t", "uintptr_t", "size_t"):
            return "int"
        if t in ("float", "double"):
            return "float"
        if t == "bool":
            return "bool"
        return "Any"

    # Add function declarations to _setup_functions
    for func in functions:
        python_code += f"        # {func['name']}\n"
        argtypes = []
        for param in func['params']:
            argtypes.append(resolve_ctype(param['type']))
        python_code += f"        self.lib.{func['name']}.argtypes = [{', '.join(argtypes)}]\n"
        return_type = func['return_type']
        python_code += f"        self.lib.{func['name']}.restype = {resolve_ctype(return_type)}\n"

    # Generate wrapper methods for each function
    for func in functions:
        func_name = func['name']
        params = func['params']

        # Extract parameter names and types
        param_names = []
        param_types = []
        param_docs = []

        for param in params:
            param_type = param['type']
            param_name = param['name']

            param_names.append(param_name)
            param_types.append(param_type)

            # Create documentation for the parameter
            param_docs.append(f"            {param_name} ({param['full']}): Parameter")

        # Convert C types to Python types for type hints
        python_types = []
        for param_type in param_types:
            python_types.append(ctype_to_pytype(param_type))

        # Generate function signature with type hints
        return_pytype = ctype_to_pytype(func['return_type'])
        python_code += f"""
    def {func_name}(self, {', '.join([f'{name}: {type}' for name, type in zip(param_names, python_types)])}) -> {return_pytype}:
        \"\"\"
        Wrapper for {func_name} function.

        Args:
{chr(10).join(param_docs)}

        Returns:
            {return_pytype} result from the C function
        \"\"\"
        return self.lib.{func_name}({', '.join(param_names)})
"""

    python_code += """
if __name__ == "__main__":
    # Example usage
    econf = EconfManager()
    print("EconfManager interface initialized successfully")
"""

    return python_code

def main():
    """Main function to read header file and generate Python interface."""
    if len(sys.argv) != 3:
        print("Usage: python generate_python_interface.py <header_file> <output_file>")
        sys.exit(1)

    header_file = sys.argv[1]
    output_file = sys.argv[2]

    try:
        with open(header_file, 'r') as f:
            header_content = f.read()

        enums, structs, typedefs, functions, typedef_map = parse_header(header_content)
        python_code = generate_python_interface(enums, structs, typedefs, functions, typedef_map)

        with open(output_file, 'w') as f:
            f.write(python_code)

        print(f"Successfully generated Python interface in {output_file}")

    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()
