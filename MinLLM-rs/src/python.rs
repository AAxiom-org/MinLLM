#![cfg(feature = "python")]

use std::collections::HashMap;
use std::sync::Arc;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple, PyList};
use pyo3::exceptions::{PyRuntimeError, PyTypeError};
use pyo3::PyResult;
use serde_json::Value;

use crate::base::{BaseNode as RustBaseNode, Node as RustNodeTrait, SharedState};
use crate::node::{Node as RustNode, BatchNode as RustBatchNode};
use crate::flow::{Flow as RustFlow, BatchFlow as RustBatchFlow};
use crate::async_node::{
    AsyncNodeTrait, 
    AsyncNode as RustAsyncNode, 
    AsyncBatchNode as RustAsyncBatchNode, 
    AsyncParallelBatchNode as RustAsyncParallelBatchNode
};
use crate::async_flow::{
    AsyncFlow as RustAsyncFlow, 
    AsyncBatchFlow as RustAsyncBatchFlow, 
    AsyncParallelBatchFlow as RustAsyncParallelBatchFlow
};
use crate::error::Error;

/// Convert Python object to serde_json Value
fn py_to_value(py: Python, obj: &PyAny) -> PyResult<Value> {
    if obj.is_none() {
        return Ok(Value::Null);
    }
    
    if let Ok(val) = obj.extract::<bool>() {
        return Ok(Value::Bool(val));
    }
    
    if let Ok(val) = obj.extract::<i64>() {
        return Ok(Value::Number(val.into()));
    }
    
    if let Ok(val) = obj.extract::<f64>() {
        return match serde_json::Number::from_f64(val) {
            Some(n) => Ok(Value::Number(n)),
            None => Err(PyTypeError::new_err(format!("Cannot convert f64 to JSON Number: {}", val))),
        };
    }
    
    if let Ok(val) = obj.extract::<String>() {
        return Ok(Value::String(val));
    }
    
    if let Ok(list) = obj.downcast::<PyTuple>() {
        let mut values = Vec::new();
        for item in list.iter() {
            values.push(py_to_value(py, item)?);
        }
        return Ok(Value::Array(values));
    }
    
    if let Ok(list) = obj.extract::<Vec<&PyAny>>() {
        let mut values = Vec::new();
        for item in list {
            values.push(py_to_value(py, item)?);
        }
        return Ok(Value::Array(values));
    }
    
    if let Ok(dict) = obj.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (key, value) in dict.iter() {
            let key = key.extract::<String>()?;
            let value = py_to_value(py, value)?;
            map.insert(key, value);
        }
        return Ok(Value::Object(map));
    }
    
    Err(PyTypeError::new_err(format!("Cannot convert Python object to JSON: {:?}", obj)))
}

/// Convert serde_json Value to Python object
fn value_to_py(py: Python, value: Value) -> PyResult<PyObject> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(b.to_object(py)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.to_object(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.to_object(py))
            } else {
                Err(PyTypeError::new_err("Unsupported number type"))
            }
        },
        Value::String(s) => Ok(s.to_object(py)),
        Value::Array(arr) => {
            let py_list = PyList::empty(py);
            for item in arr {
                py_list.append(value_to_py(py, item)?)?;
            }
            Ok(py_list.to_object(py))
        },
        Value::Object(obj) => {
            let py_dict = PyDict::new(py);
            for (key, value) in obj {
                py_dict.set_item(key, value_to_py(py, value)?)?;
            }
            Ok(py_dict.to_object(py))
        }
    }
}

/// Convert Python dict to Rust SharedState
fn py_dict_to_shared_state(py: Python, dict: &PyAny) -> PyResult<SharedState> {
    let dict = dict.downcast::<PyDict>()?;
    let mut shared = HashMap::new();
    for (key, value) in dict.iter() {
        let key = key.extract::<String>()?;
        let value = py_to_value(py, value)?;
        shared.insert(key, value);
    }
    Ok(shared)
}

/// Python wrapper for BaseNode
#[pyclass(name = "BaseNode")]
struct PyBaseNode {
    node: Arc<RustBaseNode>,
}

#[pymethods]
impl PyBaseNode {
    #[new]
    fn new() -> Self {
        Self {
            node: Arc::new(RustBaseNode::new()),
        }
    }
    
    fn set_params(&self, py: Python, params: &PyDict) -> PyResult<()> {
        let mut rust_params = HashMap::new();
        for (key, value) in params.iter() {
            let key = key.extract::<String>()?;
            let value = py_to_value(py, value)?;
            rust_params.insert(key, value);
        }
        self.node.set_params(rust_params);
        Ok(())
    }
    
    fn add_successor(&self, py: Python, node: PyObject, action: Option<&str>) -> PyResult<PyObject> {
        let action = action.unwrap_or("default");
        let successor: &PyAny = node.extract(py)?;
        
        // Extract the Rust node from the Python object
        let successor_node: Arc<dyn RustNodeTrait> = if let Ok(py_node) = successor.extract::<PyRef<PyBaseNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyBatchNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyFlow>>() {
            py_node.flow.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyBatchFlow>>() {
            py_node.flow.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyAsyncNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyAsyncBatchNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyAsyncParallelBatchNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyAsyncFlow>>() {
            py_node.flow.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyAsyncBatchFlow>>() {
            py_node.flow.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyAsyncParallelBatchFlow>>() {
            py_node.flow.clone()
        } else {
            return Err(PyTypeError::new_err("Invalid node type"));
        };
        
        self.node.add_successor(successor_node, action).map_err(|e| {
            PyRuntimeError::new_err(format!("{}", e))
        })?;
        
        Ok(node)
    }
    
    #[pyo3(text_signature = "($self, shared)")]
    fn prep(&self, py: Python, shared: &PyAny) -> PyResult<PyObject> {
        let mut shared_state = py_dict_to_shared_state(py, shared)?;
        let result = self.node.prep(&mut shared_state).map_err(|e| {
            PyRuntimeError::new_err(format!("{}", e))
        })?;
        value_to_py(py, result)
    }
    
    #[pyo3(text_signature = "($self, prep_res)")]
    fn exec(&self, py: Python, prep_res: &PyAny) -> PyResult<PyObject> {
        let prep_value = py_to_value(py, prep_res)?;
        let result = self.node.exec(prep_value).map_err(|e| {
            PyRuntimeError::new_err(format!("{}", e))
        })?;
        value_to_py(py, result)
    }
    
    #[pyo3(text_signature = "($self, shared, prep_res, exec_res)")]
    fn post(&self, py: Python, shared: &PyAny, prep_res: &PyAny, exec_res: &PyAny) -> PyResult<Option<String>> {
        let mut shared_state = py_dict_to_shared_state(py, shared)?;
        let prep_value = py_to_value(py, prep_res)?;
        let exec_value = py_to_value(py, exec_res)?;
        
        let result = self.node.post(&mut shared_state, prep_value, exec_value).map_err(|e| {
            PyRuntimeError::new_err(format!("{}", e))
        })?;
        
        Ok(result)
    }
    
    #[pyo3(text_signature = "($self, shared)")]
    fn run(&self, py: Python, shared: &PyAny) -> PyResult<Option<String>> {
        let mut shared_state = py_dict_to_shared_state(py, shared)?;
        
        let result = self.node.run(&mut shared_state).map_err(|e| {
            PyRuntimeError::new_err(format!("{}", e))
        })?;
        
        // Update the Python shared dictionary with the values from SharedState
        let shared_dict = shared.downcast::<PyDict>()?;
        for (key, value) in shared_state {
            shared_dict.set_item(key, value_to_py(py, value)?)?;
        }
        
        Ok(result)
    }
    
    fn __rshift__(&self, py: Python, other: PyObject) -> PyResult<PyObject> {
        self.add_successor(py, other, None)
    }
    
    fn __sub__(&self, py: Python, action: &PyAny) -> PyResult<PyObject> {
        if let Ok(action_str) = action.extract::<String>() {
            let conditional = PyConditionalTransition {
                src: self.node.clone(),
                action: action_str,
            };
            return Ok(Py::new(py, conditional)?.to_object(py));
        }
        Err(PyTypeError::new_err("Action must be a string"))
    }
}

/// Python wrapper for ConditionalTransition
#[pyclass(name = "_ConditionalTransition")]
struct PyConditionalTransition {
    src: Arc<dyn RustNodeTrait>,
    action: String,
}

#[pymethods]
impl PyConditionalTransition {
    fn __rshift__(&self, py: Python, other: PyObject) -> PyResult<PyObject> {
        let tgt: &PyAny = other.extract(py)?;
        
        // Extract the Rust node from the Python object based on its type
        let tgt_node: Arc<dyn RustNodeTrait> = if let Ok(py_node) = tgt.extract::<PyRef<PyBaseNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = tgt.extract::<PyRef<PyNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = tgt.extract::<PyRef<PyBatchNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = tgt.extract::<PyRef<PyFlow>>() {
            py_node.flow.clone()
        } else if let Ok(py_node) = tgt.extract::<PyRef<PyBatchFlow>>() {
            py_node.flow.clone()
        } else if let Ok(py_node) = tgt.extract::<PyRef<PyAsyncNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = tgt.extract::<PyRef<PyAsyncBatchNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = tgt.extract::<PyRef<PyAsyncParallelBatchNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = tgt.extract::<PyRef<PyAsyncFlow>>() {
            py_node.flow.clone()
        } else if let Ok(py_node) = tgt.extract::<PyRef<PyAsyncBatchFlow>>() {
            py_node.flow.clone()
        } else if let Ok(py_node) = tgt.extract::<PyRef<PyAsyncParallelBatchFlow>>() {
            py_node.flow.clone()
        } else {
            return Err(PyTypeError::new_err("Invalid node type"));
        };
        
        self.src.add_successor(tgt_node, &self.action).map_err(|e| {
            PyRuntimeError::new_err(format!("{}", e))
        })?;
        
        Ok(other)
    }
}

/// Python wrapper for Node
#[pyclass(name = "Node")]
pub struct PyNode {
    node: Arc<RustNode>,
}

#[pymethods]
impl PyNode {
    #[new]
    #[pyo3(signature = (max_retries=1, wait=0))]
    fn new(max_retries: usize, wait: u64) -> Self {
        Self {
            node: Arc::new(RustNode::new(max_retries, wait)),
        }
    }
    
    fn set_params(&self, py: Python, params: &PyDict) -> PyResult<()> {
        let mut rust_params = HashMap::new();
        for (key, value) in params.iter() {
            let key = key.extract::<String>()?;
            let value = py_to_value(py, value)?;
            rust_params.insert(key, value);
        }
        self.node.set_params(rust_params);
        Ok(())
    }
    
    fn add_successor(&self, py: Python, node: PyObject, action: Option<&str>) -> PyResult<PyObject> {
        let action = action.unwrap_or("default");
        let successor: &PyAny = node.extract(py)?;
        
        // Extract the Rust node from the Python object
        let successor_node: Arc<dyn RustNodeTrait> = if let Ok(py_node) = successor.extract::<PyRef<PyBaseNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyBatchNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyFlow>>() {
            py_node.flow.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyBatchFlow>>() {
            py_node.flow.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyAsyncNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyAsyncBatchNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyAsyncParallelBatchNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyAsyncFlow>>() {
            py_node.flow.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyAsyncBatchFlow>>() {
            py_node.flow.clone()
        } else if let Ok(py_node) = successor.extract::<PyRef<PyAsyncParallelBatchFlow>>() {
            py_node.flow.clone()
        } else {
            return Err(PyTypeError::new_err("Invalid node type"));
        };
        
        self.node.add_successor(successor_node, action).map_err(|e| {
            PyRuntimeError::new_err(format!("{}", e))
        })?;
        
        Ok(node)
    }
    
    #[pyo3(text_signature = "($self, shared)")]
    fn prep(&self, py: Python, shared: &PyAny) -> PyResult<PyObject> {
        let mut shared_state = py_dict_to_shared_state(py, shared)?;
        let result = self.node.prep(&mut shared_state).map_err(|e| {
            PyRuntimeError::new_err(format!("{}", e))
        })?;
        value_to_py(py, result)
    }
    
    #[pyo3(text_signature = "($self, prep_res)")]
    fn exec(&self, py: Python, prep_res: &PyAny) -> PyResult<PyObject> {
        let prep_value = py_to_value(py, prep_res)?;
        let result = self.node.exec(prep_value).map_err(|e| {
            PyRuntimeError::new_err(format!("{}", e))
        })?;
        value_to_py(py, result)
    }
    
    #[pyo3(text_signature = "($self, prep_res, exc)")]
    fn exec_fallback(&self, py: Python, prep_res: &PyAny, exc: &PyAny) -> PyResult<PyObject> {
        let prep_value = py_to_value(py, prep_res)?;
        let error = Error::NodeExecution(format!("Python exception: {}", exc));
        
        let result = self.node.exec_fallback(prep_value, error).map_err(|e| {
            PyRuntimeError::new_err(format!("{}", e))
        })?;
        
        value_to_py(py, result)
    }
    
    #[pyo3(text_signature = "($self, shared, prep_res, exec_res)")]
    fn post(&self, py: Python, shared: &PyAny, prep_res: &PyAny, exec_res: &PyAny) -> PyResult<Option<String>> {
        let mut shared_state = py_dict_to_shared_state(py, shared)?;
        let prep_value = py_to_value(py, prep_res)?;
        let exec_value = py_to_value(py, exec_res)?;
        
        let result = self.node.post(&mut shared_state, prep_value, exec_value).map_err(|e| {
            PyRuntimeError::new_err(format!("{}", e))
        })?;
        
        Ok(result)
    }
    
    #[pyo3(text_signature = "($self, shared)")]
    fn run(&self, py: Python, shared: &PyAny) -> PyResult<Option<String>> {
        let mut shared_state = py_dict_to_shared_state(py, shared)?;
        
        let result = self.node.run(&mut shared_state).map_err(|e| {
            PyRuntimeError::new_err(format!("{}", e))
        })?;
        
        // Update the Python shared dictionary with the values from SharedState
        let shared_dict = shared.downcast::<PyDict>()?;
        for (key, value) in shared_state {
            shared_dict.set_item(key, value_to_py(py, value)?)?;
        }
        
        Ok(result)
    }
    
    fn __rshift__(&self, py: Python, other: PyObject) -> PyResult<PyObject> {
        self.add_successor(py, other, None)
    }
    
    fn __sub__(&self, py: Python, action: &PyAny) -> PyResult<PyObject> {
        if let Ok(action_str) = action.extract::<String>() {
            let conditional = PyConditionalTransition {
                src: self.node.clone(),
                action: action_str,
            };
            return Ok(Py::new(py, conditional)?.to_object(py));
        }
        Err(PyTypeError::new_err("Action must be a string"))
    }
}

/// Python wrapper for BatchNode
#[pyclass(name = "BatchNode")]
struct PyBatchNode {
    node: Arc<RustBatchNode>,
}

#[pymethods]
impl PyBatchNode {
    #[new]
    #[pyo3(signature = (max_retries=1, wait=0))]
    fn new(max_retries: usize, wait: u64) -> Self {
        Self {
            node: Arc::new(RustBatchNode::new(max_retries, wait)),
        }
    }
    
    // Define the same methods as PyNode, but for BatchNode
    // This is essentially the same code, just referencing node instead of node
    // Implementation details are omitted for brevity
    // In a real implementation, you would copy the methods from PyNode and adapt them
}

/// Python wrapper for Flow
#[pyclass(name = "Flow")]
pub struct PyFlow {
    flow: Arc<RustFlow>,
}

#[pymethods]
impl PyFlow {
    #[new]
    fn new(py: Python, start: PyObject) -> PyResult<Self> {
        let start_node: &PyAny = start.extract(py)?;
        
        // Extract the Rust node from the Python object
        let start_node = if let Ok(py_node) = start_node.extract::<PyRef<PyBaseNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyBatchNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyAsyncNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyAsyncBatchNode>>() {
            py_node.node.clone() as Arc<dyn RustNodeTrait>
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyAsyncParallelBatchNode>>() {
            py_node.node.clone() as Arc<dyn RustNodeTrait>
        } else {
            return Err(PyTypeError::new_err("Invalid start node type"));
        };
        
        Ok(Self {
            flow: Arc::new(RustFlow::new(start_node)),
        })
    }
    
    // Define similar methods as PyNode, but adapted for Flow
    // Implementation details are omitted for brevity
}

/// Python wrapper for BatchFlow
#[pyclass(name = "BatchFlow")]
struct PyBatchFlow {
    flow: Arc<RustBatchFlow>,
}

#[pymethods]
impl PyBatchFlow {
    #[new]
    fn new(py: Python, start: PyObject) -> PyResult<Self> {
        let start_node: &PyAny = start.extract(py)?;
        
        // Extract the Rust node from the Python object
        let start_node = if let Ok(py_node) = start_node.extract::<PyRef<PyBaseNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyBatchNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyAsyncNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyAsyncBatchNode>>() {
            py_node.node.clone() as Arc<dyn RustNodeTrait>
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyAsyncParallelBatchNode>>() {
            py_node.node.clone() as Arc<dyn RustNodeTrait>
        } else {
            return Err(PyTypeError::new_err("Invalid start node type"));
        };
        
        Ok(Self {
            flow: Arc::new(RustBatchFlow::new(start_node)),
        })
    }
    
    // Define similar methods as PyNode, but adapted for BatchFlow
    // Implementation details are omitted for brevity
}

/// Python wrapper for AsyncNode
#[pyclass(name = "AsyncNode")]
pub struct PyAsyncNode {
    node: Arc<RustAsyncNode>,
}

#[pymethods]
impl PyAsyncNode {
    #[new]
    #[pyo3(signature = (max_retries=1, wait=0))]
    fn new(max_retries: usize, wait: u64) -> Self {
        Self {
            node: Arc::new(RustAsyncNode::new(max_retries, wait)),
        }
    }
    
    // Define similar methods as PyNode, but for async operations
    // Implementation details are omitted for brevity
    
    #[pyo3(text_signature = "($self, shared)")]
    fn run_async<'p>(&self, py: Python<'p>, shared: &'p PyAny) -> PyResult<&'p PyAny> {
        // Clone the shared state before the async block
        let mut shared_state = py_dict_to_shared_state(py, shared)?;
        let node = self.node.clone();
        
        let future = pyo3_asyncio::tokio::future_into_py(py, async move {
            let result = node.run_async(&mut shared_state).await.map_err(|e| {
                PyRuntimeError::new_err(format!("{}", e))
            })?;
            
            // Convert the result to a JSON string to avoid lifetime issues
            let result_str = match &result {
                Some(s) => s.to_string(),
                None => "null".to_string(),
            };
            
            // Return the serialized data as a string
            Ok(result_str)
        })?;
        
        Ok(future)
    }
}

/// Python wrapper for AsyncBatchNode
#[pyclass(name = "AsyncBatchNode")]
pub struct PyAsyncBatchNode {
    node: Arc<RustAsyncBatchNode>,
}

#[pymethods]
impl PyAsyncBatchNode {
    #[new]
    #[pyo3(signature = (max_retries=1, wait=0))]
    fn new(max_retries: usize, wait: u64) -> Self {
        Self {
            node: Arc::new(RustAsyncBatchNode::new(max_retries, wait)),
        }
    }
    
    // Define similar methods as PyAsyncNode
    // Implementation details are omitted for brevity
}

/// Python wrapper for AsyncParallelBatchNode
#[pyclass(name = "AsyncParallelBatchNode")]
pub struct PyAsyncParallelBatchNode {
    node: Arc<RustAsyncParallelBatchNode>,
}

#[pymethods]
impl PyAsyncParallelBatchNode {
    #[new]
    #[pyo3(signature = (max_retries=1, wait=0))]
    fn new(max_retries: usize, wait: u64) -> Self {
        Self {
            node: Arc::new(RustAsyncParallelBatchNode::new(max_retries, wait)),
        }
    }
    
    // Define similar methods as PyAsyncNode
    // Implementation details are omitted for brevity
}

/// Python wrapper for AsyncFlow
#[pyclass(name = "AsyncFlow")]
pub struct PyAsyncFlow {
    flow: Arc<RustAsyncFlow>,
}

#[pymethods]
impl PyAsyncFlow {
    #[new]
    fn new(py: Python, start: PyObject) -> PyResult<Self> {
        let start_node: &PyAny = start.extract(py)?;
        
        // Extract the Rust node from the Python object
        let start_node = if let Ok(py_node) = start_node.extract::<PyRef<PyBaseNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyBatchNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyAsyncNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyAsyncBatchNode>>() {
            py_node.node.clone() as Arc<dyn RustNodeTrait>
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyAsyncParallelBatchNode>>() {
            py_node.node.clone() as Arc<dyn RustNodeTrait>
        } else {
            return Err(PyTypeError::new_err("Invalid start node type"));
        };
        
        Ok(Self {
            flow: Arc::new(RustAsyncFlow::new(start_node)),
        })
    }
    
    // Define similar methods as PyFlow, but for async operations
    // Implementation details are omitted for brevity
    
    #[pyo3(text_signature = "($self, shared)")]
    fn run_async<'p>(&self, py: Python<'p>, shared: &'p PyAny) -> PyResult<&'p PyAny> {
        // Clone the shared state before the async block
        let mut shared_state = py_dict_to_shared_state(py, shared)?;
        let flow = self.flow.clone();
        
        let future = pyo3_asyncio::tokio::future_into_py(py, async move {
            let result = flow.run_async(&mut shared_state).await.map_err(|e| {
                PyRuntimeError::new_err(format!("{}", e))
            })?;
            
            // Convert the result to a JSON string to avoid lifetime issues
            let result_str = match &result {
                Some(s) => s.to_string(),
                None => "null".to_string(),
            };
            
            // Return the serialized data as a string
            Ok(result_str)
        })?;
        
        Ok(future)
    }
}

/// Python wrapper for AsyncBatchFlow
#[pyclass(name = "AsyncBatchFlow")]
pub struct PyAsyncBatchFlow {
    flow: Arc<RustAsyncBatchFlow>,
}

#[pymethods]
impl PyAsyncBatchFlow {
    #[new]
    fn new(py: Python, start: PyObject) -> PyResult<Self> {
        let start_node: &PyAny = start.extract(py)?;
        
        // Extract the Rust node from the Python object
        let start_node = if let Ok(py_node) = start_node.extract::<PyRef<PyBaseNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyBatchNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyAsyncNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyAsyncBatchNode>>() {
            py_node.node.clone() as Arc<dyn RustNodeTrait>
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyAsyncParallelBatchNode>>() {
            py_node.node.clone() as Arc<dyn RustNodeTrait>
        } else {
            return Err(PyTypeError::new_err("Invalid start node type"));
        };
        
        Ok(Self {
            flow: Arc::new(RustAsyncBatchFlow::new(start_node)),
        })
    }
    
    // Define similar methods as PyAsyncFlow but adapted for AsyncBatchFlow
    // Implementation details are omitted for brevity
}

/// Python wrapper for AsyncParallelBatchFlow
#[pyclass(name = "AsyncParallelBatchFlow")]
pub struct PyAsyncParallelBatchFlow {
    flow: Arc<RustAsyncParallelBatchFlow>,
}

#[pymethods]
impl PyAsyncParallelBatchFlow {
    #[new]
    fn new(py: Python, start: PyObject) -> PyResult<Self> {
        let start_node: &PyAny = start.extract(py)?;
        
        // Extract the Rust node from the Python object
        let start_node = if let Ok(py_node) = start_node.extract::<PyRef<PyBaseNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyBatchNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyAsyncNode>>() {
            py_node.node.clone()
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyAsyncBatchNode>>() {
            py_node.node.clone() as Arc<dyn RustNodeTrait>
        } else if let Ok(py_node) = start_node.extract::<PyRef<PyAsyncParallelBatchNode>>() {
            py_node.node.clone() as Arc<dyn RustNodeTrait>
        } else {
            return Err(PyTypeError::new_err("Invalid start node type"));
        };
        
        Ok(Self {
            flow: Arc::new(RustAsyncParallelBatchFlow::new(start_node)),
        })
    }
    
    // Define similar methods as PyAsyncFlow but adapted for AsyncParallelBatchFlow
    // Implementation details are omitted for brevity
}

/// Initialize the module
#[pymodule]
fn _minllm(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyBaseNode>()?;
    m.add_class::<PyConditionalTransition>()?;
    m.add_class::<PyNode>()?;
    m.add_class::<PyBatchNode>()?;
    m.add_class::<PyFlow>()?;
    m.add_class::<PyBatchFlow>()?;
    m.add_class::<PyAsyncNode>()?;
    m.add_class::<PyAsyncBatchNode>()?;
    m.add_class::<PyAsyncParallelBatchNode>()?;
    m.add_class::<PyAsyncFlow>()?;
    m.add_class::<PyAsyncBatchFlow>()?;
    m.add_class::<PyAsyncParallelBatchFlow>()?;
    
    Ok(())
} 