use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;

use crate::error::{ActionName, MinLLMError, Result};
use crate::node::{Node, BaseNode, ParamMap};
use crate::store::SharedStore;

/// Flow is a container for a series of connected nodes
pub struct Flow {
    base: BaseNode,
    start: Box<dyn Node>,
}

impl Flow {
    pub fn new(start: Box<dyn Node>) -> Self {
        Self {
            base: BaseNode::new(),
            start,
        }
    }
    
    /// Get the next node in the flow based on the current action
    fn get_next_node(&self, current: &dyn Node, action: &str) -> Option<Box<dyn Node>> {
        let action_name = action.to_string();
        let default_action = "default".to_string();
        
        // Try to get the successor for the specific action
        if let Some(next) = current.get_successor(&action_name) {
            // Clone the next node
            return Some(clone_box(next));
        }
        
        // If not found and action is not default, try default
        if action_name != default_action {
            if let Some(next) = current.get_successor(&default_action) {
                return Some(clone_box(next));
            }
        }
        
        // If no successor found and there are successors, warn
        if !action_name.is_empty() {
            eprintln!("Warning: Flow ends: '{}' not found", action_name);
        }
        
        None
    }
    
    /// Orchestrate the flow execution
    fn orchestrate(&self, shared: &SharedStore, params: Option<ParamMap>) {
        let mut current = Some(clone_box(&self.start));
        let params = params.unwrap_or_else(|| self.base.params.clone());
        
        while let Some(mut node) = current {
            node.set_params(params.clone());
            let action = node.run(shared);
            current = self.get_next_node(&*node, &action.0);
        }
    }
    
    /// Run the flow
    pub fn run(&self, shared: &SharedStore) -> ActionName {
        let prep_result = self.base.prep(shared);
        self.orchestrate(shared, None);
        self.base.post(shared, prep_result, Box::new(()))
    }
    
    /// Async version of orchestrate
    async fn orchestrate_async(&self, shared: &SharedStore, params: Option<ParamMap>) {
        let mut current = Some(clone_box(&self.start));
        let params = params.unwrap_or_else(|| self.base.params.clone());
        
        while let Some(mut node) = current {
            node.set_params(params.clone());
            
            // Check if the node is async-capable and call the appropriate method
            let action = if let Some(async_node) = node.as_any().downcast_ref::<AsyncNode>() {
                async_node.run_async(shared).await
            } else {
                node.run(shared)
            };
            
            current = self.get_next_node(&*node, &action.0);
        }
    }
    
    /// Async version of run
    pub async fn run_async(&self, shared: &SharedStore) -> ActionName {
        let prep_result = self.base.prep(shared);
        self.orchestrate_async(shared, None).await;
        self.base.post(shared, prep_result, Box::new(()))
    }
}

#[async_trait]
impl Node for Flow {
    fn set_params(&mut self, params: ParamMap) {
        self.base.set_params(params.clone());
        self.start.set_params(params);
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
    
    fn exec(&self, _prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        panic!("Flow cannot exec directly. Use run() instead.");
    }
    
    fn post(&self, shared: &SharedStore, prep_result: Box<dyn Any + Send + Sync>, 
            exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        self.base.post(shared, prep_result, exec_result)
    }
}

/// BatchFlow processes a batch of parameters
pub struct BatchFlow {
    flow: Flow,
}

impl BatchFlow {
    pub fn new(start: Box<dyn Node>) -> Self {
        Self {
            flow: Flow::new(start),
        }
    }
}

#[async_trait]
impl Node for BatchFlow {
    fn set_params(&mut self, params: ParamMap) {
        self.flow.set_params(params);
    }
    
    fn add_successor(&mut self, node: Box<dyn Node>, action: impl Into<ActionName>) -> &mut Self {
        self.flow.add_successor(node, action);
        self
    }
    
    fn get_successor(&self, action: &str) -> Option<&Box<dyn Node>> {
        self.flow.get_successor(action)
    }
    
    fn prep(&self, shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        self.flow.prep(shared)
    }
    
    fn exec(&self, _prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        panic!("BatchFlow cannot exec directly. Use run() instead.");
    }
    
    fn post(&self, shared: &SharedStore, prep_result: Box<dyn Any + Send + Sync>, 
            exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        self.flow.post(shared, prep_result, exec_result)
    }
    
    fn run(&self, shared: &SharedStore) -> ActionName {
        let prep_result = self.prep(shared);
        
        // Try to downcast to Vec<ParamMap>
        if let Some(batch_params) = prep_result.downcast_ref::<Vec<ParamMap>>() {
            for params in batch_params {
                self.flow.orchestrate(shared, Some(params.clone()));
            }
        }
        
        self.post(shared, prep_result, Box::new(()))
    }
}

/// Helper trait to allow downcast of Node trait objects
pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
}

impl<T: Any> AsAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Clone a boxed Node
fn clone_box(node: &Box<dyn Node>) -> Box<dyn Node> {
    // This is a placeholder. In practice, we'd need to implement a Clone trait 
    // for all Node implementations, or use a factory pattern.
    // For now, we'll leave it as an unimplemented placeholder
    unimplemented!("Node cloning not yet implemented")
}

/// AsyncNode trait for nodes that support async operations
pub trait AsyncNode: Node {
    async fn prep_async(&self, shared: &SharedStore) -> Box<dyn Any + Send + Sync>;
    async fn exec_async(&self, prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync>;
    async fn post_async(&self, shared: &SharedStore, prep_result: Box<dyn Any + Send + Sync>,
                        exec_result: Box<dyn Any + Send + Sync>) -> ActionName;
    async fn run_async(&self, shared: &SharedStore) -> ActionName;
} 