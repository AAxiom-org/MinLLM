use pyo3::prelude::*;
use pyo3::wrap_pyfunction;

mod conversions;
mod node;
mod flow;
mod store;

use node::{PyBaseNode, PyNode, PyBatchNode, PyAsyncNode, PyAsyncBatchNode, PyAsyncParallelBatchNode};
use flow::{PyFlow, PyBatchFlow, PyAsyncFlow, PyAsyncBatchFlow, PyAsyncParallelBatchFlow};
use store::PySharedStore;

#[pymodule]
fn minllm(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PySharedStore>()?;
    m.add_class::<PyBaseNode>()?;
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