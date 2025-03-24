use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;

use crate::error::{ActionName, MinLLMError, Result};
use crate::node::{Node, NodeMut, BaseNode, ParamMap};
use crate::store::SharedStore;

/// Flow is a container for a series of connected nodes
pub struct Flow {
    pub(crate) base: BaseNode,
    pub(crate) start: Box<dyn Node>,
}

impl Clone for Flow {
    fn clone(&self) -> Self {
        // Since we can't clone Box<dyn Node>, we create a new Flow with same base params
        // but without successors. This is for use in Python bindings where we'll handle
        // cloning specially.
        Self {
            base: self.base.clone(),
            start: new_placeholder_node(),
        }
    }
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
            return Some(deep_clone_node(next));
        }
        
        // If not found and action is not default, try default
        if action_name != default_action {
            if let Some(next) = current.get_successor(&default_action) {
                return Some(deep_clone_node(next));
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
        let mut current = Some(deep_clone_node(&self.start));
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
        let mut current = Some(deep_clone_node(&self.start));
        let params = params.unwrap_or_else(|| self.base.params.clone());
        
        while let Some(mut node) = current {
            node.set_params(params.clone());
            
            // Check if the node is async-capable and call the appropriate method
            let action = if let Some(async_node) = node.as_any().downcast_ref::<dyn AsyncNode>() {
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
    
    fn get_successor(&self, action: &str) -> Option<&Box<dyn Node>> {
        self.base.get_successor(action)
    }
    
    fn prep(&self, shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        self.base.prep(shared)
    }
    
    fn exec(&self, _prep_result: &Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        panic!("Flow cannot exec directly. Use run() instead.");
    }
    
    fn post(&self, shared: &SharedStore, prep_result: &Box<dyn Any + Send + Sync>, 
            exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        self.base.post(shared, prep_result, exec_result)
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl NodeMut for Flow {
    fn add_successor(&mut self, node: Box<dyn Node>, action: impl Into<ActionName>) -> &mut Self {
        self.base.add_successor(node, action);
        self
    }
}

/// BatchFlow processes a batch of parameters
pub struct BatchFlow {
    flow: Flow,
}

impl Clone for BatchFlow {
    fn clone(&self) -> Self {
        Self {
            flow: self.flow.clone(),
        }
    }
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
    
    fn get_successor(&self, action: &str) -> Option<&Box<dyn Node>> {
        self.flow.get_successor(action)
    }
    
    fn prep(&self, shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        self.flow.prep(shared)
    }
    
    fn exec(&self, _prep_result: &Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        panic!("BatchFlow cannot exec directly. Use run() instead.");
    }
    
    fn post(&self, shared: &SharedStore, prep_result: &Box<dyn Any + Send + Sync>, 
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
    
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl NodeMut for BatchFlow {
    fn add_successor(&mut self, node: Box<dyn Node>, action: impl Into<ActionName>) -> &mut Self {
        self.flow.add_successor(node, action);
        self
    }
}

/// AsyncNode trait for nodes that support async operations
#[async_trait]
pub trait AsyncNode: Node {
    async fn prep_async(&self, shared: &SharedStore) -> Box<dyn Any + Send + Sync>;
    async fn exec_async(&self, prep_result: &Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync>;
    async fn post_async(&self, shared: &SharedStore, prep_result: &Box<dyn Any + Send + Sync>,
                       exec_result: Box<dyn Any + Send + Sync>) -> ActionName;
    async fn run_async(&self, shared: &SharedStore) -> ActionName;
}

/// Function to create a deep clone of a node
/// This is a placeholder implementation - in a real system, you'd need a proper
/// factory pattern or other mechanism to clone trait objects
pub fn deep_clone_node(node: &Box<dyn Node>) -> Box<dyn Node> {
    // Since Box<dyn Node> doesn't implement Clone, we create a placeholder
    new_placeholder_node()
}

// Adding this clone_box function that was referenced but missing
pub fn clone_box<T: 'static + Clone>(boxed: &Box<dyn Any + Send + Sync>) -> Option<T> {
    match boxed.downcast_ref::<T>() {
        Some(value) => Some(value.clone()),
        None => None,
    }
}

fn new_placeholder_node() -> Box<dyn Node> {
    Box::new(BaseNode::new())
} 