use pyo3::prelude::*;
use pyo3::exceptions::PyValueError;
use std::sync::Arc;

use crate::store::SharedStore;
use super::conversions::{py_to_any, any_to_py};

/// Python wrapper for SharedStore
#[pyclass(name = "SharedStore")]
pub struct PySharedStore {
    inner: SharedStore,
}

#[pymethods]
impl PySharedStore {
    #[new]
    fn new() -> Self {
        Self {
            inner: SharedStore::new(),
        }
    }
    
    /// Get a value from the store by key
    fn get(&self, py: Python, key: &str) -> PyResult<PyObject> {
        match self.inner.get::<Box<dyn std::any::Any + Send + Sync>>(key) {
            Some(value) => any_to_py(py, &value),
            None => Ok(py.None()),
        }
    }
    
    /// Set a value in the store
    fn set(&self, key: &str, value: &PyAny) -> PyResult<()> {
        let any_value = py_to_any(value)?;
        self.inner.set(key, any_value);
        Ok(())
    }
    
    /// Remove a value from the store
    fn remove(&self, key: &str) -> bool {
        self.inner.remove(key)
    }
    
    /// Check if a key exists in the store
    fn contains_key(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }
    
    /// Get all keys in the store
    fn keys(&self, py: Python) -> PyObject {
        let keys = self.inner.keys();
        keys.to_object(py)
    }
    
    /// Create a clone of this store
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
    
    /// String representation of the store
    fn __repr__(&self) -> String {
        format!("SharedStore(keys={})", self.inner.keys().len())
    }
    
    /// String representation of the store
    fn __str__(&self) -> String {
        self.__repr__()
    }
} 