use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::error::{MinLLMError, Result};

/// SharedStore is a thread-safe key-value store that can hold any types
/// that are Send + Sync. It's used for communication between nodes.
pub struct SharedStore {
    data: Arc<RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>>,
}

impl SharedStore {
    /// Create a new, empty SharedStore
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get a value from the store by key
    /// Returns None if the key doesn't exist or the type doesn't match
    pub fn get<T: 'static + Clone + Send + Sync>(&self, key: &str) -> Option<T> {
        let data = self.data.read();
        data.get(key).and_then(|boxed| {
            boxed.downcast_ref::<T>().cloned()
        })
    }

    /// Set a value in the store
    pub fn set<T: 'static + Send + Sync>(&self, key: &str, value: T) {
        let mut data = self.data.write();
        data.insert(key.to_string(), Box::new(value));
    }

    /// Remove a value from the store
    pub fn remove(&self, key: &str) -> bool {
        let mut data = self.data.write();
        data.remove(key).is_some()
    }

    /// Check if a key exists in the store
    pub fn contains_key(&self, key: &str) -> bool {
        let data = self.data.read();
        data.contains_key(key)
    }

    /// Get all keys in the store
    pub fn keys(&self) -> Vec<String> {
        let data = self.data.read();
        data.keys().cloned().collect()
    }

    /// Create a clone of this store
    pub fn clone(&self) -> Self {
        let data = self.data.read();
        let cloned_data = data.clone();
        Self {
            data: Arc::new(RwLock::new(cloned_data)),
        }
    }
}

impl Default for SharedStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for SharedStore {
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
        }
    }
} 