use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::types::{PyDict, PyType};

use std::collections::HashMap;
use std::sync::Arc;
use std::any::Any;

use crate::flow::{Flow as RustFlow, BatchFlow as RustBatchFlow, AsyncNode};
use crate::async_flow::{AsyncFlow as RustAsyncFlow, AsyncBatchFlow as RustAsyncBatchFlow, AsyncParallelBatchFlow as RustAsyncParallelBatchFlow};
use crate::store::SharedStore;
use crate::error::ActionName;

use super::store::PySharedStore;
use super::node::{PyBaseNode, PyNode, PyAsyncNode};
use super::conversions::{py_to_any, any_to_py, py_to_param_map};

/// Flow orchestrates a series of nodes
#[pyclass(name = "Flow", extends = PyBaseNode)]
pub struct PyFlow {
    start: PyObject,
}

#[pymethods]
impl PyFlow {
    #[new]
    fn new(py: Python, start: PyObject) -> (Self, PyBaseNode) {
        (
            Self {
                start,
            },
            PyBaseNode::new()
        )
    }
    
    /// Get the next node in the flow based on the current action
    fn get_next_node(&self, py: Python, current: &PyAny, action: Option<&str>) -> PyResult<Option<PyObject>> {
        let action_name = action.unwrap_or("default").to_string();
        
        // Try to get successors attribute
        if let Ok(successors) = current.getattr("successors") {
            // Try to get the successor for the given action
            if let Ok(successor) = successors.get_item(action_name.clone()) {
                return Ok(Some(successor.to_object(py)));
            }
            
            // If not found and action is not default, try default
            if action_name != "default" {
                if let Ok(successor) = successors.get_item("default") {
                    return Ok(Some(successor.to_object(py)));
                }
            }
            
            // If there are no successors for the given action, warn
            if successors.len()? > 0 {
                eprintln!("Warning: Flow ends: '{}' not found", action_name);
            }
        }
        
        Ok(None)
    }
    
    /// Execute the flow
    fn exec(&self, _prep_result: &PyAny) -> PyResult<PyObject> {
        Err(PyRuntimeError::new_err("Flow can't exec."))
    }
    
    /// Orchestrate the flow execution
    fn _orch(&self, py: Python, shared: &PySharedStore, params: Option<&PyDict>) -> PyResult<()> {
        let mut current = Some(self.start.clone_ref(py));
        
        // Get the base node
        let base = PyBaseNode::extract(py, self.into_py(py))?;
        
        // Create params
        let p = if let Some(params_dict) = params {
            params_dict.to_object(py)
        } else {
            // Get params from base node
            if let Ok(base_params) = PyDict::new(py).extract::<&PyDict>() {
                for (key, value) in base.params.iter() {
                    base_params.set_item(key, value.clone_ref(py))?;
                }
                base_params.to_object(py)
            } else {
                PyDict::new(py).to_object(py)
            }
        };
        
        while let Some(node) = current {
            let node_ref = node.as_ref(py);
            
            // Set params
            node_ref.call_method1("set_params", (p.clone_ref(py),))?;
            
            // Run the node
            let action = node_ref.call_method0("_run")?.extract::<String>()?;
            
            // Get the next node
            current = self.get_next_node(py, node_ref, Some(&action))?;
        }
        
        Ok(())
    }
    
    /// Run the flow
    fn _run(&self, py: Python, shared: &PySharedStore) -> PyResult<String> {
        // Get the base node
        let base = PyBaseNode::extract(py, self.into_py(py))?;
        
        // Prepare
        let prep_result = base.prep(shared)?;
        
        // Orchestrate the flow
        self._orch(py, shared, None)?;
        
        // Post
        base.post(shared, &prep_result, &prep_result)
    }
}

/// BatchFlow processes a batch of parameters
#[pyclass(name = "BatchFlow", extends = PyFlow)]
pub struct PyBatchFlow {
    // No additional fields needed
}

#[pymethods]
impl PyBatchFlow {
    #[new]
    fn new(py: Python, start: PyObject) -> (Self, PyFlow, PyBaseNode) {
        (
            Self {},
            PyFlow {
                start,
            },
            PyBaseNode::new()
        )
    }
    
    /// Run the batch flow
    fn _run(&self, py: Python, shared: &PySharedStore) -> PyResult<String> {
        // Get the base node and flow
        let base = PyBaseNode::extract(py, self.into_py(py))?;
        let flow = PyFlow::extract(py, self.into_py(py))?;
        
        // Prepare (get batch params)
        let prep_result = base.prep(shared)?;
        
        // If prep_result can be iterated, process each as params
        if let Ok(iter) = prep_result.iter() {
            for item in iter {
                let item = item?;
                if let Ok(params) = item.downcast::<PyDict>() {
                    flow._orch(py, shared, Some(params))?;
                }
            }
        }
        
        // Post
        base.post(shared, &prep_result, &prep_result)
    }
}

/// AsyncFlow orchestrates nodes asynchronously
#[pyclass(name = "AsyncFlow", extends = PyFlow)]
pub struct PyAsyncFlow {
    // No additional fields needed
}

#[pymethods]
impl PyAsyncFlow {
    #[new]
    fn new(py: Python, start: PyObject) -> (Self, PyFlow, PyBaseNode) {
        (
            Self {},
            PyFlow {
                start,
            },
            PyBaseNode::new()
        )
    }
    
    /// Execute the flow
    fn exec(&self, _prep_result: &PyAny) -> PyResult<PyObject> {
        Err(PyRuntimeError::new_err("AsyncFlow can't exec."))
    }
    
    /// Orchestrate the flow execution asynchronously
    fn _orch_async(&self, py: Python, shared: &PySharedStore, params: Option<&PyDict>) -> PyResult<()> {
        // This will be an async function in Python
        // For now, just delegate to the sync version
        let flow = PyFlow::extract(py, self.into_py(py))?;
        flow._orch(py, shared, params)
    }
    
    /// Run the flow asynchronously
    fn run_async(&self, py: Python, shared: &PySharedStore) -> PyResult<String> {
        // This will be an async function in Python
        self._run_async(py, shared)
    }
    
    /// Internal async run method
    fn _run_async(&self, py: Python, shared: &PySharedStore) -> PyResult<String> {
        // This will be an async function in Python
        // Get the base node
        let base = PyBaseNode::extract(py, self.into_py(py))?;
        
        // In the Python version, these will be async calls
        let prep_result = base.prep(shared)?;
        self._orch_async(py, shared, None)?;
        base.post(shared, &prep_result, &prep_result)
    }
}

/// AsyncBatchFlow processes batches asynchronously
#[pyclass(name = "AsyncBatchFlow", extends = PyAsyncFlow)]
pub struct PyAsyncBatchFlow {
    // No additional fields needed
}

#[pymethods]
impl PyAsyncBatchFlow {
    #[new]
    fn new(py: Python, start: PyObject) -> (Self, PyAsyncFlow, PyFlow, PyBaseNode) {
        (
            Self {},
            PyAsyncFlow {},
            PyFlow {
                start,
            },
            PyBaseNode::new()
        )
    }
    
    /// Run the batch flow asynchronously
    fn _run_async(&self, py: Python, shared: &PySharedStore) -> PyResult<String> {
        // This will be an async function in Python
        // Get the base node and flow
        let base = PyBaseNode::extract(py, self.into_py(py))?;
        let flow = PyAsyncFlow::extract(py, self.into_py(py))?;
        
        // In the Python version, these will be async calls
        let prep_result = base.prep(shared)?;
        
        // If prep_result can be iterated, process each as params
        if let Ok(iter) = prep_result.iter() {
            for item in iter {
                let item = item?;
                if let Ok(params) = item.downcast::<PyDict>() {
                    flow._orch_async(py, shared, Some(params))?;
                }
            }
        }
        
        base.post(shared, &prep_result, &prep_result)
    }
}

/// AsyncParallelBatchFlow processes batches in parallel asynchronously
#[pyclass(name = "AsyncParallelBatchFlow", extends = PyAsyncFlow)]
pub struct PyAsyncParallelBatchFlow {
    // No additional fields needed
}

#[pymethods]
impl PyAsyncParallelBatchFlow {
    #[new]
    fn new(py: Python, start: PyObject) -> (Self, PyAsyncFlow, PyFlow, PyBaseNode) {
        (
            Self {},
            PyAsyncFlow {},
            PyFlow {
                start,
            },
            PyBaseNode::new()
        )
    }
    
    /// Run the batch flow asynchronously in parallel
    fn _run_async(&self, py: Python, shared: &PySharedStore) -> PyResult<String> {
        // This will be an async function in Python that uses asyncio.gather
        // For now, just use the sequential version
        let async_flow = PyAsyncBatchFlow::extract(py, PyAsyncBatchFlow::new(py, self.as_ref(py).getattr("start")?.clone()).into_py(py))?;
        async_flow._run_async(py, shared)
    }
} 