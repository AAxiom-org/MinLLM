use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::fmt;
use std::time::Duration;
use std::thread;
use async_trait::async_trait;
use parking_lot::RwLock;

use crate::error::{ActionName, MinLLMError, Result};
use crate::store::SharedStore;

// Generic type for parameters
pub type ParamMap = HashMap<String, serde_json::Value>;

// Node trait definition
#[async_trait]
pub trait Node: Send + Sync {
    /// Prepare phase - gathers data from the shared store
    fn prep(&self, shared: &SharedStore) -> Box<dyn Any + Send + Sync>;
    
    /// Execute phase - performs the main computation
    fn exec(&self, prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync>;
    
    /// Post phase - stores results and returns the next action
    fn post(&self, shared: &SharedStore, prep_result: Box<dyn Any + Send + Sync>, 
            exec_result: Box<dyn Any + Send + Sync>) -> ActionName;
    
    /// Set parameters for this node
    fn set_params(&mut self, params: ParamMap);
    
    /// Add a successor node for a given action
    fn add_successor(&mut self, node: Box<dyn Node>, action: impl Into<ActionName>) -> &mut Self;
    
    /// Get a successor node for a given action
    fn get_successor(&self, action: &str) -> Option<&Box<dyn Node>>;
    
    /// Run the node (combines prep, exec, and post)
    fn run(&self, shared: &SharedStore) -> ActionName {
        let prep_result = self.prep(shared);
        let exec_result = self._exec(prep_result.clone());
        self.post(shared, prep_result, exec_result)
    }
    
    /// Internal execution method (overridden by derived nodes)
    fn _exec(&self, prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        self.exec(prep_result)
    }
    
    // Async versions
    async fn prep_async(&self, shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        self.prep(shared)
    }
    
    async fn exec_async(&self, prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        self.exec(prep_result)
    }
    
    async fn post_async(&self, shared: &SharedStore, prep_result: Box<dyn Any + Send + Sync>,
                       exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        self.post(shared, prep_result, exec_result)
    }
    
    async fn run_async(&self, shared: &SharedStore) -> ActionName {
        let prep_result = self.prep_async(shared).await;
        let exec_result = self._exec_async(prep_result.clone()).await;
        self.post_async(shared, prep_result, exec_result).await
    }
    
    async fn _exec_async(&self, prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        self.exec_async(prep_result).await
    }
}

// Base implementation for all nodes
pub struct BaseNode {
    params: ParamMap,
    successors: HashMap<String, Box<dyn Node>>,
}

impl BaseNode {
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
            successors: HashMap::new(),
        }
    }
}

impl Default for BaseNode {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Node for BaseNode {
    fn set_params(&mut self, params: ParamMap) {
        self.params = params;
    }
    
    fn add_successor(&mut self, node: Box<dyn Node>, action: impl Into<ActionName>) -> &mut Self {
        let action_name = action.into();
        if self.successors.contains_key(&action_name.0) {
            eprintln!("Warning: Overwriting successor for action '{}'", action_name);
        }
        self.successors.insert(action_name.0, node);
        self
    }
    
    fn get_successor(&self, action: &str) -> Option<&Box<dyn Node>> {
        self.successors.get(action)
    }
    
    fn prep(&self, _shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        Box::new(())
    }
    
    fn exec(&self, _prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        Box::new(())
    }
    
    fn post(&self, _shared: &SharedStore, _prep_result: Box<dyn Any + Send + Sync>, 
            _exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        ActionName::default()
    }
}

// Regular Node with retry capabilities
pub struct RegularNode {
    base: BaseNode,
    max_retries: usize,
    wait: u64,
    current_retry: usize,
}

impl RegularNode {
    pub fn new(max_retries: usize, wait: u64) -> Self {
        Self {
            base: BaseNode::new(),
            max_retries,
            wait,
            current_retry: 0,
        }
    }
    
    pub fn exec_fallback(&self, _prep_result: Box<dyn Any + Send + Sync>, exc: Box<dyn std::error::Error + Send + Sync>) 
        -> Box<dyn Any + Send + Sync> {
        // Default implementation just re-raises the exception
        panic!("Node execution failed after {} retries: {:?}", self.max_retries, exc);
    }
}

#[async_trait]
impl Node for RegularNode {
    fn set_params(&mut self, params: ParamMap) {
        self.base.set_params(params);
    }
    
    fn add_successor(&mut self, node: Box<dyn Node>, action: impl Into<ActionName>) -> &mut Self {
        self.base.add_successor(node, action);
        self
    }
    
    fn get_successor(&self, action: &str) -> Option<&Box<dyn Node>> {
        self.base.get_successor(action)
    }
    
    fn prep(&self, shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        self.base.prep(shared)
    }
    
    fn exec(&self, prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        self.base.exec(prep_result)
    }
    
    fn post(&self, shared: &SharedStore, prep_result: Box<dyn Any + Send + Sync>, 
            exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        self.base.post(shared, prep_result, exec_result)
    }
    
    fn _exec(&self, prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        let mut retry_count = 0;
        loop {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.exec(prep_result.clone())
            })) {
                Ok(result) => return result,
                Err(err) => {
                    retry_count += 1;
                    if retry_count >= self.max_retries {
                        let error = if let Some(e) = err.downcast_ref::<&dyn std::error::Error>() {
                            format!("{}", e)
                        } else if let Some(s) = err.downcast_ref::<&str>() {
                            s.to_string()
                        } else {
                            "Unknown error".to_string()
                        };
                        
                        let boxed_error = Box::new(MinLLMError::NodeError(error)) as Box<dyn std::error::Error + Send + Sync>;
                        return self.exec_fallback(prep_result.clone(), boxed_error);
                    }
                    
                    if self.wait > 0 {
                        thread::sleep(Duration::from_millis(self.wait));
                    }
                }
            }
        }
    }
}

// BatchNode for processing batches of items
pub struct BatchNode {
    node: RegularNode,
}

impl BatchNode {
    pub fn new(max_retries: usize, wait: u64) -> Self {
        Self {
            node: RegularNode::new(max_retries, wait),
        }
    }
}

#[async_trait]
impl Node for BatchNode {
    fn set_params(&mut self, params: ParamMap) {
        self.node.set_params(params);
    }
    
    fn add_successor(&mut self, node: Box<dyn Node>, action: impl Into<ActionName>) -> &mut Self {
        self.node.add_successor(node, action);
        self
    }
    
    fn get_successor(&self, action: &str) -> Option<&Box<dyn Node>> {
        self.node.get_successor(action)
    }
    
    fn prep(&self, shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        self.node.prep(shared)
    }
    
    fn exec(&self, prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        self.node.exec(prep_result)
    }
    
    fn post(&self, shared: &SharedStore, prep_result: Box<dyn Any + Send + Sync>, 
            exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        self.node.post(shared, prep_result, exec_result)
    }
    
    fn _exec(&self, prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        // Try to downcast to Vec<T>
        if let Some(items) = prep_result.downcast_ref::<Vec<Box<dyn Any + Send + Sync>>>() {
            let results = items.iter()
                .map(|item| self.node._exec(item.clone()))
                .collect::<Vec<_>>();
            Box::new(results)
        } else {
            // If not a Vec, process as a single item
            self.node._exec(prep_result)
        }
    }
} 