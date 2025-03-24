use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use serde_json::Value;
use log::warn;

use crate::base::{BaseNode, Node as NodeTrait};
use crate::error::{Error, Result};

/// A node with retry capability
#[derive(Clone)]
pub struct Node {
    /// The base node implementation
    base: BaseNode,
    
    /// Maximum number of retries
    max_retries: usize,
    
    /// Wait time between retries in milliseconds
    wait: u64,
    
    /// Current retry count
    cur_retry: Arc<RwLock<usize>>,
}

impl Node {
    /// Create a new node with retry capability
    pub fn new(max_retries: usize, wait: u64) -> Self {
        Self {
            base: BaseNode::new(),
            max_retries,
            wait,
            cur_retry: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Called on execution failure, can be overridden
    pub fn exec_fallback(&self, _prep_res: Value, error: Error) -> Result<Value> {
        Err(error)
    }
}

impl Default for Node {
    fn default() -> Self {
        Self::new(1, 0)
    }
}

impl NodeTrait for Node {
    fn params(&self) -> Arc<RwLock<HashMap<String, Value>>> {
        self.base.params()
    }
    
    fn successors(&self) -> Arc<RwLock<HashMap<String, Arc<dyn NodeTrait>>>> {
        self.base.successors()
    }
    
    fn set_params(&self, params: HashMap<String, Value>) {
        let params_lock = self.params();
        let mut p = params_lock.write().unwrap();
        *p = params;
    }
    
    fn add_successor(&self, node: Arc<dyn NodeTrait>, action: &str) -> Result<Arc<dyn NodeTrait>> {
        let successors_lock = self.successors();
        let mut successors = successors_lock.write().unwrap();
        if successors.contains_key(action) {
            warn!("Overwriting successor for action '{}'", action);
        }
        successors.insert(action.to_string(), node.clone());
        Ok(node)
    }
    
    fn _exec(&self, prep_res: Value) -> Result<Value> {
        for retry in 0..self.max_retries {
            {
                let mut cur_retry = self.cur_retry.write().unwrap();
                *cur_retry = retry;
            }
            
            match self.exec(prep_res.clone()) {
                Ok(res) => return Ok(res),
                Err(e) => {
                    if retry == self.max_retries - 1 {
                        return self.exec_fallback(prep_res, e);
                    }
                    
                    if self.wait > 0 {
                        thread::sleep(Duration::from_millis(self.wait));
                    }
                }
            }
        }
        
        // This should never happen if max_retries > 0
        Err(Error::NodeExecution("Max retries exceeded".into()))
    }
}

/// A node that processes batches of items
#[derive(Clone)]
pub struct BatchNode {
    /// The underlying node
    node: Node,
}

impl BatchNode {
    /// Create a new batch node
    pub fn new(max_retries: usize, wait: u64) -> Self {
        Self {
            node: Node::new(max_retries, wait),
        }
    }
}

impl Default for BatchNode {
    fn default() -> Self {
        Self::new(1, 0)
    }
}

impl NodeTrait for BatchNode {
    fn params(&self) -> Arc<RwLock<HashMap<String, Value>>> {
        self.node.params()
    }
    
    fn successors(&self) -> Arc<RwLock<HashMap<String, Arc<dyn NodeTrait>>>> {
        self.node.successors()
    }
    
    fn set_params(&self, params: HashMap<String, Value>) {
        self.node.set_params(params);
    }
    
    fn add_successor(&self, node: Arc<dyn NodeTrait>, action: &str) -> Result<Arc<dyn NodeTrait>> {
        self.node.add_successor(node, action)
    }
    
    fn _exec(&self, items: Value) -> Result<Value> {
        // Handle empty batches
        if items.is_null() {
            return Ok(Value::Array(vec![]));
        }
        
        // Ensure we have an array
        let items = match items {
            Value::Array(items) => items,
            _ => return Err(Error::NodeExecution("BatchNode requires an array".into())),
        };
        
        // Process each item using the node's exec method
        let mut results = Vec::with_capacity(items.len());
        for item in items {
            let result = self.node._exec(item)?;
            results.push(result);
        }
        
        Ok(Value::Array(results))
    }
} 