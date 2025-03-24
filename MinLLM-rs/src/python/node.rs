use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::types::{PyDict, PyType};

use std::collections::HashMap;
use std::sync::Arc;
use std::any::Any;

use crate::node::{BaseNode as RustBaseNode, RegularNode as RustNode, BatchNode as RustBatchNode, ParamMap};
use crate::async_node::{AsyncNodeImpl as RustAsyncNode, AsyncBatchNode as RustAsyncBatchNode, AsyncParallelBatchNode as RustAsyncParallelBatchNode};
use crate::store::SharedStore;
use crate::error::ActionName;

use super::store::PySharedStore;
use super::conversions::{py_to_any, any_to_py, py_to_param_map};

/// Conditional Transition for Python's `node - "action" >> other_node` syntax
#[pyclass(name = "_ConditionalTransition")]
pub struct PyConditionalTransition {
    source: PyObject,
    action: String,
}

#[pymethods]
impl PyConditionalTransition {
    #[new]
    fn new(source: PyObject, action: String) -> Self {
        Self { source, action }
    }
    
    /// Right-shift operator implementation
    fn __rshift__(&self, py: Python, target: PyObject) -> PyResult<PyObject> {
        // Call the add_successor method on the source node
        let source = self.source.as_ref(py);
        let args = (target, &self.action);
        source.call_method1("add_successor", args)
    }
}

/// Python BaseNode class
#[pyclass(name = "BaseNode")]
pub struct PyBaseNode {
    params: HashMap<String, PyObject>,
    successors: HashMap<String, PyObject>,
}

#[pymethods]
impl PyBaseNode {
    #[new]
    fn new() -> Self {
        Self {
            params: HashMap::new(),
            successors: HashMap::new(),
        }
    }
    
    /// Set parameters for this node
    fn set_params(&mut self, params: &PyDict) -> PyResult<()> {
        self.params.clear();
        for (key, value) in params.iter() {
            let key_str = key.extract::<String>()?;
            let value_obj = value.to_object(params.py());
            self.params.insert(key_str, value_obj);
        }
        Ok(())
    }
    
    /// Add a successor node for the given action
    fn add_successor(&mut self, node: PyObject, action: Option<&str>) -> PyResult<PyObject> {
        let action_name = action.unwrap_or("default").to_string();
        
        if self.successors.contains_key(&action_name) {
            eprintln!("Warning: Overwriting successor for action '{}'", action_name);
        }
        
        self.successors.insert(action_name, node.clone());
        Ok(node)
    }
    
    /// Preparation phase
    fn prep(&self, _shared: &PySharedStore) -> PyResult<PyObject> {
        Ok(_shared.py().None())
    }
    
    /// Execution phase
    fn exec(&self, _prep_result: &PyAny) -> PyResult<PyObject> {
        Ok(_prep_result.py().None())
    }
    
    /// Post-execution phase
    fn post(&self, _shared: &PySharedStore, _prep_result: &PyAny, _exec_result: &PyAny) -> PyResult<String> {
        Ok("default".to_string())
    }
    
    /// Internal execution method
    fn _exec(&self, prep_result: &PyAny) -> PyResult<PyObject> {
        self.exec(prep_result)
    }
    
    /// Run the node
    fn run(&self, shared: &PySharedStore) -> PyResult<String> {
        let py = shared.py();
        
        if !self.successors.is_empty() {
            eprintln!("Warning: Node won't run successors. Use Flow.");
        }
        
        self._run(shared)
    }
    
    /// Internal run method
    fn _run(&self, shared: &PySharedStore) -> PyResult<String> {
        let py = shared.py();
        let prep_result = self.prep(shared)?;
        let exec_result = self._exec(&prep_result)?;
        self.post(shared, &prep_result, &exec_result)
    }
    
    /// Right-shift operator (for node >> other_node syntax)
    fn __rshift__(&self, py: Python, other: PyObject) -> PyResult<PyObject> {
        let args = (other,);
        let result = self.add_successor(args.0, None)?;
        Ok(result)
    }
    
    /// Subtraction operator (for node - "action" syntax)
    fn __sub__(&self, py: Python, action: &PyAny) -> PyResult<PyObject> {
        if let Ok(action_str) = action.extract::<String>() {
            let cls = py.import("minllm")?.getattr("_ConditionalTransition")?;
            let args = (self.into_py(py), action_str);
            let obj = cls.call1(args)?;
            return Ok(obj.to_object(py));
        }
        
        Err(PyValueError::new_err("Action must be a string"))
    }
    
    fn __repr__(&self) -> String {
        format!("BaseNode(params={}, successors={})", self.params.len(), self.successors.len())
    }
    
    fn __str__(&self) -> String {
        self.__repr__()
    }
}

/// Regular Node with retry capabilities
#[pyclass(name = "Node", extends = PyBaseNode)]
pub struct PyNode {
    max_retries: usize,
    wait: u64,
    cur_retry: usize,
}

#[pymethods]
impl PyNode {
    #[new]
    #[pyo3(signature = (max_retries=1, wait=0))]
    fn new(max_retries: usize, wait: u64) -> (Self, PyBaseNode) {
        (
            Self {
                max_retries,
                wait,
                cur_retry: 0,
            },
            PyBaseNode::new()
        )
    }
    
    /// Fallback execution method for when regular execution fails
    fn exec_fallback(&self, py: Python, prep_result: &PyAny, exc: PyErr) -> PyResult<PyObject> {
        // Default implementation just re-raises the exception
        Err(exc)
    }
    
    /// Internal execution method with retry logic
    fn _exec(&self, py: Python, prep_result: &PyAny) -> PyResult<PyObject> {
        let base = PyBaseNode::extract(py, self.into_py(py))?;
        
        for retry in 0..self.max_retries {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.cur_retry = retry;
                base.exec(prep_result)
            })) {
                Ok(result) => return result,
                Err(_) => {
                    // If we've used all retries, call the fallback
                    if retry == self.max_retries - 1 {
                        let error = PyRuntimeError::new_err("Execution failed");
                        return self.exec_fallback(py, prep_result, error);
                    }
                    
                    // Otherwise wait and try again
                    if self.wait > 0 {
                        std::thread::sleep(std::time::Duration::from_millis(self.wait));
                    }
                }
            }
        }
        
        // This should never happen due to the loop above
        Err(PyRuntimeError::new_err("Execution failed after retries"))
    }
}

/// BatchNode for processing batches of items
#[pyclass(name = "BatchNode", extends = PyNode)]
pub struct PyBatchNode {
    // No additional fields needed
}

#[pymethods]
impl PyBatchNode {
    #[new]
    #[pyo3(signature = (max_retries=1, wait=0))]
    fn new(max_retries: usize, wait: u64) -> (Self, PyNode, PyBaseNode) {
        (
            Self {},
            PyNode {
                max_retries,
                wait,
                cur_retry: 0,
            },
            PyBaseNode::new()
        )
    }
    
    /// Process batch items
    fn _exec(&self, py: Python, items: &PyAny) -> PyResult<PyObject> {
        // Get the base node instance
        let node = PyNode::extract(py, self.into_py(py))?;
        
        // Handle the case when items is None
        if items.is_none() {
            return Ok(py.None());
        }
        
        // Try to iterate through the items
        let iter = items.iter()?;
        let mut results = Vec::new();
        
        for item in iter {
            let result = node._exec(py, &item?)?;
            results.push(result);
        }
        
        // Convert the results to a Python list
        Ok(results.to_object(py))
    }
}

/// Async Node for asynchronous operations
#[pyclass(name = "AsyncNode", extends = PyNode)]
pub struct PyAsyncNode {
    // No additional fields needed
}

#[pymethods]
impl PyAsyncNode {
    #[new]
    #[pyo3(signature = (max_retries=1, wait=0))]
    fn new(max_retries: usize, wait: u64) -> (Self, PyNode, PyBaseNode) {
        (
            Self {},
            PyNode {
                max_retries,
                wait,
                cur_retry: 0,
            },
            PyBaseNode::new()
        )
    }
    
    // Blocking methods should raise an error
    fn prep(&self, _shared: &PySharedStore) -> PyResult<PyObject> {
        Err(PyRuntimeError::new_err("Use prep_async"))
    }
    
    fn exec(&self, _prep_result: &PyAny) -> PyResult<PyObject> {
        Err(PyRuntimeError::new_err("Use exec_async"))
    }
    
    fn post(&self, _shared: &PySharedStore, _prep_result: &PyAny, _exec_result: &PyAny) -> PyResult<String> {
        Err(PyRuntimeError::new_err("Use post_async"))
    }
    
    fn exec_fallback(&self, _py: Python, _prep_result: &PyAny, _exc: PyErr) -> PyResult<PyObject> {
        Err(PyRuntimeError::new_err("Use exec_fallback_async"))
    }
    
    fn _run(&self, _shared: &PySharedStore) -> PyResult<String> {
        Err(PyRuntimeError::new_err("Use run_async"))
    }
    
    // Async methods (these will be called from Python)
    fn prep_async(&self, py: Python, _shared: &PySharedStore) -> PyResult<PyObject> {
        Ok(py.None())
    }
    
    fn exec_async(&self, py: Python, _prep_result: &PyAny) -> PyResult<PyObject> {
        Ok(py.None())
    }
    
    fn exec_fallback_async(&self, py: Python, _prep_result: &PyAny, exc: PyErr) -> PyResult<PyObject> {
        Err(exc)
    }
    
    fn post_async(&self, _shared: &PySharedStore, _prep_result: &PyAny, _exec_result: &PyAny) -> PyResult<String> {
        Ok("default".to_string())
    }
    
    fn _exec_async(&self, py: Python, prep_result: &PyAny) -> PyResult<PyObject> {
        // In Python, this will be an async function that uses the retry logic
        self.exec_async(py, prep_result)
    }
    
    fn run_async(&self, py: Python, shared: &PySharedStore) -> PyResult<String> {
        if !PyBaseNode::extract(py, self.into_py(py))?.successors.is_empty() {
            eprintln!("Warning: Node won't run successors. Use AsyncFlow.");
        }
        
        self._run_async(shared)
    }
    
    fn _run_async(&self, py: Python, shared: &PySharedStore) -> PyResult<String> {
        // In Python, this will be an async function
        let prep_result = self.prep_async(py, shared)?;
        let exec_result = self._exec_async(py, &prep_result)?;
        self.post_async(shared, &prep_result, &exec_result)
    }
}

/// AsyncBatchNode for asynchronous batch processing
#[pyclass(name = "AsyncBatchNode", extends=PyAsyncNode)]
pub struct PyAsyncBatchNode {
    // No additional fields needed
}

#[pymethods]
impl PyAsyncBatchNode {
    #[new]
    #[pyo3(signature = (max_retries=1, wait=0))]
    fn new(max_retries: usize, wait: u64) -> (Self, PyAsyncNode, PyNode, PyBaseNode) {
        (
            Self {},
            PyAsyncNode {},
            PyNode {
                max_retries,
                wait,
                cur_retry: 0,
            },
            PyBaseNode::new()
        )
    }
    
    /// Process batch items asynchronously
    fn _exec_async(&self, py: Python, items: &PyAny) -> PyResult<PyObject> {
        // In Python, this will be an async function that processes items sequentially
        // For now, just return the items as-is
        Ok(items.to_object(py))
    }
}

/// AsyncParallelBatchNode for parallel asynchronous batch processing
#[pyclass(name = "AsyncParallelBatchNode", extends=PyAsyncNode)]
pub struct PyAsyncParallelBatchNode {
    // No additional fields needed
}

#[pymethods]
impl PyAsyncParallelBatchNode {
    #[new]
    #[pyo3(signature = (max_retries=1, wait=0))]
    fn new(max_retries: usize, wait: u64) -> (Self, PyAsyncNode, PyNode, PyBaseNode) {
        (
            Self {},
            PyAsyncNode {},
            PyNode {
                max_retries,
                wait,
                cur_retry: 0,
            },
            PyBaseNode::new()
        )
    }
    
    /// Process batch items asynchronously in parallel
    fn _exec_async(&self, py: Python, items: &PyAny) -> PyResult<PyObject> {
        // In Python, this will be an async function that uses asyncio.gather
        // For now, just return the items as-is
        Ok(items.to_object(py))
    }
} 