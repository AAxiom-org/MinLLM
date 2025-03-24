use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use std::collections::HashMap;
use std::any::Any;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::error::{MinLLMError, Result, ActionName};
use crate::node::ParamMap;

// Convert a Python object to a ParamMap
pub fn py_to_param_map(obj: &PyAny) -> PyResult<ParamMap> {
    let dict = obj.downcast::<PyDict>()?;
    let mut params = HashMap::new();
    
    for (key, value) in dict.iter() {
        let key_str = key.extract::<String>()?;
        let json_value = py_to_json(value)?;
        params.insert(key_str, json_value);
    }
    
    Ok(params)
}

// Convert a ParamMap to a Python dict
pub fn param_map_to_py(py: Python, params: &ParamMap) -> PyResult<PyObject> {
    let dict = PyDict::new(py);
    
    for (key, value) in params {
        let py_value = json_to_py(py, value)?;
        dict.set_item(key, py_value)?;
    }
    
    Ok(dict.into())
}

// Convert a Python object to serde_json::Value
pub fn py_to_json(obj: &PyAny) -> PyResult<serde_json::Value> {
    if obj.is_none() {
        return Ok(serde_json::Value::Null);
    }
    
    if let Ok(val) = obj.extract::<bool>() {
        return Ok(serde_json::Value::Bool(val));
    }
    
    if let Ok(val) = obj.extract::<i64>() {
        return Ok(serde_json::Value::Number(val.into()));
    }
    
    if let Ok(val) = obj.extract::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(val) {
            return Ok(serde_json::Value::Number(num));
        }
    }
    
    if let Ok(val) = obj.extract::<String>() {
        return Ok(serde_json::Value::String(val));
    }
    
    if let Ok(list) = obj.downcast::<PyList>() {
        let mut values = Vec::new();
        for item in list.iter() {
            values.push(py_to_json(item)?);
        }
        return Ok(serde_json::Value::Array(values));
    }
    
    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (key, value) in dict.iter() {
            let key_str = key.extract::<String>()?;
            map.insert(key_str, py_to_json(value)?);
        }
        return Ok(serde_json::Value::Object(map));
    }
    
    // If it's not a standard type, stringify it
    Ok(serde_json::Value::String(obj.str()?.extract::<String>()?))
}

// Convert a serde_json::Value to a Python object
pub fn json_to_py(py: Python, value: &serde_json::Value) -> PyResult<PyObject> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(b) => Ok(b.to_object(py)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.to_object(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.to_object(py))
            } else {
                Ok(n.to_string().to_object(py))
            }
        },
        serde_json::Value::String(s) => Ok(s.to_object(py)),
        serde_json::Value::Array(a) => {
            let list = PyList::empty(py);
            for item in a {
                list.append(json_to_py(py, item)?)?;
            }
            Ok(list.into())
        },
        serde_json::Value::Object(o) => {
            let dict = PyDict::new(py);
            for (key, value) in o {
                dict.set_item(key, json_to_py(py, value)?)?;
            }
            Ok(dict.into())
        }
    }
}

// Convert a Box<dyn Any + Send + Sync> to a Python object
pub fn any_to_py(py: Python, value: &Box<dyn Any + Send + Sync>) -> PyResult<PyObject> {
    // Try common types
    if let Some(s) = value.downcast_ref::<String>() {
        return Ok(s.to_object(py));
    }
    
    if let Some(b) = value.downcast_ref::<bool>() {
        return Ok(b.to_object(py));
    }
    
    if let Some(i) = value.downcast_ref::<i64>() {
        return Ok(i.to_object(py));
    }
    
    if let Some(f) = value.downcast_ref::<f64>() {
        return Ok(f.to_object(py));
    }
    
    if let Some(json) = value.downcast_ref::<serde_json::Value>() {
        return json_to_py(py, json);
    }
    
    if let Some(map) = value.downcast_ref::<ParamMap>() {
        return param_map_to_py(py, map);
    }
    
    if let Some(v) = value.downcast_ref::<Vec<Box<dyn Any + Send + Sync>>>() {
        let list = PyList::empty(py);
        for item in v {
            list.append(any_to_py(py, item)?)?;
        }
        return Ok(list.into());
    }
    
    if let Some(v) = value.downcast_ref::<Vec<String>>() {
        let list = PyList::empty(py);
        for item in v {
            list.append(item)?;
        }
        return Ok(list.into());
    }
    
    if let Some(v) = value.downcast_ref::<Vec<i64>>() {
        let list = PyList::empty(py);
        for item in v {
            list.append(item)?;
        }
        return Ok(list.into());
    }
    
    if let Some(v) = value.downcast_ref::<Vec<f64>>() {
        let list = PyList::empty(py);
        for item in v {
            list.append(item)?;
        }
        return Ok(list.into());
    }
    
    if let Some(a) = value.downcast_ref::<ActionName>() {
        return Ok(a.0.to_object(py));
    }
    
    // If we can't match the type, return None
    Ok(py.None())
}

// Convert a Python object to Box<dyn Any + Send + Sync>
pub fn py_to_any(obj: &PyAny) -> PyResult<Box<dyn Any + Send + Sync>> {
    if obj.is_none() {
        return Ok(Box::new(()));
    }
    
    if let Ok(val) = obj.extract::<bool>() {
        return Ok(Box::new(val));
    }
    
    if let Ok(val) = obj.extract::<i64>() {
        return Ok(Box::new(val));
    }
    
    if let Ok(val) = obj.extract::<f64>() {
        return Ok(Box::new(val));
    }
    
    if let Ok(val) = obj.extract::<String>() {
        return Ok(Box::new(val));
    }
    
    if let Ok(list) = obj.downcast::<PyList>() {
        // Try to determine the list type
        if list.is_empty() {
            return Ok(Box::new(Vec::<Box<dyn Any + Send + Sync>>::new()));
        }
        
        // First, try strings
        let mut string_vec = Vec::new();
        let mut is_string_vec = true;
        for item in list.iter() {
            if let Ok(s) = item.extract::<String>() {
                string_vec.push(s);
            } else {
                is_string_vec = false;
                break;
            }
        }
        
        if is_string_vec {
            return Ok(Box::new(string_vec));
        }
        
        // Then try numbers
        let mut int_vec = Vec::new();
        let mut is_int_vec = true;
        for item in list.iter() {
            if let Ok(i) = item.extract::<i64>() {
                int_vec.push(i);
            } else {
                is_int_vec = false;
                break;
            }
        }
        
        if is_int_vec {
            return Ok(Box::new(int_vec));
        }
        
        // Then try float
        let mut float_vec = Vec::new();
        let mut is_float_vec = true;
        for item in list.iter() {
            if let Ok(f) = item.extract::<f64>() {
                float_vec.push(f);
            } else {
                is_float_vec = false;
                break;
            }
        }
        
        if is_float_vec {
            return Ok(Box::new(float_vec));
        }
        
        // Default to Any list
        let mut any_vec = Vec::new();
        for item in list.iter() {
            any_vec.push(py_to_any(item)?);
        }
        
        return Ok(Box::new(any_vec));
    }
    
    if let Ok(dict) = obj.downcast::<PyDict>() {
        return Ok(Box::new(py_to_param_map(dict)?));
    }
    
    // If can't determine the type, convert to string
    Ok(Box::new(obj.str()?.extract::<String>()?))
} 