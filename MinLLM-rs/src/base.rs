use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde_json::Value;
use log::warn;

use crate::error::{Result};

/// Shared state that is passed between nodes in a flow
pub type SharedState = HashMap<String, Value>;

/// Action that determines the next node in a flow
pub type Action = Option<String>;

/// A base node in a workflow
#[derive(Clone)]
pub struct BaseNode {
    /// Parameters for the node
    params: Arc<RwLock<HashMap<String, Value>>>,
    
    /// Successors of this node, keyed by action
    successors: Arc<RwLock<HashMap<String, Arc<dyn Node>>>>,
}

/// Trait for node functionality
pub trait Node: Send + Sync + 'static {
    /// Get a reference to the node's parameters
    fn params(&self) -> Arc<RwLock<HashMap<String, Value>>>;
    
    /// Get a reference to the node's successors
    fn successors(&self) -> Arc<RwLock<HashMap<String, Arc<dyn Node>>>>;
    
    /// Set parameters for the node
    fn set_params(&self, params: HashMap<String, Value>);
    
    /// Add a successor node for a given action
    fn add_successor(&self, node: Arc<dyn Node>, action: &str) -> Result<Arc<dyn Node>>;
    
    /// Preparation step before execution
    fn prep(&self, _shared: &mut SharedState) -> Result<Value> {
        Ok(Value::Null)
    }
    
    /// Execute the node logic
    fn exec(&self, _prep_res: Value) -> Result<Value> {
        Ok(Value::Null)
    }
    
    /// Post-execution step
    fn post(&self, _shared: &mut SharedState, _prep_res: Value, _exec_res: Value) -> Result<Action> {
        Ok(None) // No action, end the flow
    }
    
    /// Internal execute method that can be overridden by derived nodes
    fn _exec(&self, prep_res: Value) -> Result<Value> {
        self.exec(prep_res)
    }
    
    /// Run the node
    fn _run(&self, shared: &mut SharedState) -> Result<Action> {
        let prep_res = self.prep(shared)?;
        let exec_res = self._exec(prep_res.clone())?;
        self.post(shared, prep_res, exec_res)
    }
    
    /// Run the node as a standalone (warns if there are successors)
    fn run(&self, shared: &mut SharedState) -> Result<Action> {
        let successors_lock = self.successors();
        let successors = successors_lock.read().unwrap();
        if !successors.is_empty() {
            warn!("Node won't run successors. Use Flow.");
        }
        self._run(shared)
    }
}

impl BaseNode {
    /// Create a new base node
    pub fn new() -> Self {
        Self {
            params: Arc::new(RwLock::new(HashMap::new())),
            successors: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for BaseNode {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for BaseNode {
    fn params(&self) -> Arc<RwLock<HashMap<String, Value>>> {
        self.params.clone()
    }
    
    fn successors(&self) -> Arc<RwLock<HashMap<String, Arc<dyn Node>>>> {
        self.successors.clone()
    }
    
    fn set_params(&self, params: HashMap<String, Value>) {
        let params_lock = self.params();
        let mut p = params_lock.write().unwrap();
        *p = params;
    }
    
    fn add_successor(&self, node: Arc<dyn Node>, action: &str) -> Result<Arc<dyn Node>> {
        let successors_lock = self.successors();
        let mut successors = successors_lock.write().unwrap();
        if successors.contains_key(action) {
            warn!("Overwriting successor for action '{}'", action);
        }
        successors.insert(action.to_string(), node.clone());
        Ok(node)
    }
} 