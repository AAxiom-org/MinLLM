use std::any::Any;
use std::collections::HashMap;
use async_trait::async_trait;
use futures::future;

use crate::error::{ActionName, MinLLMError, Result};
use crate::node::{Node, NodeMut, BaseNode, ParamMap};
use crate::store::SharedStore;
use crate::flow::{Flow, BatchFlow, AsyncNode};

/// AsyncFlow orchestrates async nodes
pub struct AsyncFlow {
    flow: Flow,
    async_base: BaseNode,
}

impl AsyncFlow {
    pub fn new(start: Box<dyn Node>) -> Self {
        Self {
            flow: Flow::new(start),
            async_base: BaseNode::new(),
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
    
    /// Orchestrate the async flow execution
    async fn orchestrate_async(&self, shared: &SharedStore, params: Option<ParamMap>) {
        let mut current = Some(clone_box(&self.flow.start));
        let params = params.unwrap_or_else(|| self.async_base.params.clone());
        
        while let Some(mut node) = current {
            node.set_params(params.clone());
            
            // Determine if the node is async-capable
            let action = if let Some(async_node) = node.as_any().downcast_ref::<dyn AsyncNode>() {
                async_node.run_async(shared).await
            } else {
                node.run(shared)
            };
            
            current = self.get_next_node(&*node, &action.0);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[async_trait]
impl Node for AsyncFlow {
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
    
    fn prep(&self, _shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        panic!("Use prep_async instead for AsyncFlow");
    }
    
    fn exec(&self, _prep_result: &Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        panic!("AsyncFlow cannot exec directly. Use run_async() instead.");
    }
    
    fn post(&self, _shared: &SharedStore, _prep_result: &Box<dyn Any + Send + Sync>, 
           _exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        panic!("Use post_async instead for AsyncFlow");
    }
}

impl NodeMut for AsyncFlow {
    fn add_successor(&mut self, node: Box<dyn Node>, action: impl Into<ActionName>) -> &mut Self {
        self.flow.add_successor(node, action);
        self
    }
}

#[async_trait]
impl AsyncNode for AsyncFlow {
    async fn prep_async(&self, _shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        Box::new(())
    }
    
    async fn exec_async(&self, _prep_result: &Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        panic!("AsyncFlow cannot exec_async directly. Use run_async() instead.");
    }
    
    async fn post_async(&self, _shared: &SharedStore, _prep_result: &Box<dyn Any + Send + Sync>,
                      _exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        ActionName::default()
    }
    
    async fn run_async(&self, shared: &SharedStore) -> ActionName {
        let prep_result = self.prep_async(shared).await;
        self.orchestrate_async(shared, None).await;
        self.post_async(shared, &prep_result, Box::new(())).await
    }
}

/// AsyncBatchFlow processes batches of parameters asynchronously
pub struct AsyncBatchFlow {
    async_flow: AsyncFlow,
}

impl AsyncBatchFlow {
    pub fn new(start: Box<dyn Node>) -> Self {
        Self {
            async_flow: AsyncFlow::new(start),
        }
    }
}

#[async_trait]
impl Node for AsyncBatchFlow {
    fn set_params(&mut self, params: ParamMap) {
        self.async_flow.set_params(params);
    }
    
    fn add_successor(&mut self, node: Box<dyn Node>, action: impl Into<ActionName>) -> &mut Self {
        self.async_flow.add_successor(node, action);
        self
    }
    
    fn get_successor(&self, action: &str) -> Option<&Box<dyn Node>> {
        self.async_flow.get_successor(action)
    }
    
    fn prep(&self, _shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        panic!("Use prep_async instead for AsyncBatchFlow");
    }
    
    fn exec(&self, _prep_result: &Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        panic!("AsyncBatchFlow cannot exec directly. Use run_async() instead.");
    }
    
    fn post(&self, _shared: &SharedStore, _prep_result: &Box<dyn Any + Send + Sync>, 
           _exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        panic!("Use post_async instead for AsyncBatchFlow");
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl NodeMut for AsyncBatchFlow {
    fn add_successor(&mut self, node: Box<dyn Node>, action: impl Into<ActionName>) -> &mut Self {
        self.async_flow.add_successor(node, action);
        self
    }
}

#[async_trait]
impl AsyncNode for AsyncBatchFlow {
    async fn prep_async(&self, shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        self.async_flow.prep_async(shared).await
    }
    
    async fn exec_async(&self, _prep_result: &Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        panic!("AsyncBatchFlow cannot exec_async directly. Use run_async() instead.");
    }
    
    async fn post_async(&self, shared: &SharedStore, prep_result: &Box<dyn Any + Send + Sync>,
                      exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        self.async_flow.post_async(shared, prep_result, exec_result).await
    }
    
    async fn run_async(&self, shared: &SharedStore) -> ActionName {
        let prep_result = self.prep_async(shared).await;
        
        // Process batch params sequentially
        if let Some(batch_params) = prep_result.downcast_ref::<Vec<ParamMap>>() {
            for params in batch_params {
                self.async_flow.orchestrate_async(shared, Some(params.clone())).await;
            }
        }
        
        self.post_async(shared, &prep_result, Box::new(())).await
    }
}

/// AsyncParallelBatchFlow processes batches of parameters asynchronously in parallel
pub struct AsyncParallelBatchFlow {
    async_flow: AsyncFlow,
}

impl AsyncParallelBatchFlow {
    pub fn new(start: Box<dyn Node>) -> Self {
        Self {
            async_flow: AsyncFlow::new(start),
        }
    }
}

#[async_trait]
impl Node for AsyncParallelBatchFlow {
    fn set_params(&mut self, params: ParamMap) {
        self.async_flow.set_params(params);
    }
    
    fn add_successor(&mut self, node: Box<dyn Node>, action: impl Into<ActionName>) -> &mut Self {
        self.async_flow.add_successor(node, action);
        self
    }
    
    fn get_successor(&self, action: &str) -> Option<&Box<dyn Node>> {
        self.async_flow.get_successor(action)
    }
    
    fn prep(&self, _shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        panic!("Use prep_async instead for AsyncParallelBatchFlow");
    }
    
    fn exec(&self, _prep_result: &Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        panic!("AsyncParallelBatchFlow cannot exec directly. Use run_async() instead.");
    }
    
    fn post(&self, _shared: &SharedStore, _prep_result: &Box<dyn Any + Send + Sync>, 
           _exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        panic!("Use post_async instead for AsyncParallelBatchFlow");
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl NodeMut for AsyncParallelBatchFlow {
    fn add_successor(&mut self, node: Box<dyn Node>, action: impl Into<ActionName>) -> &mut Self {
        self.async_flow.add_successor(node, action);
        self
    }
}

#[async_trait]
impl AsyncNode for AsyncParallelBatchFlow {
    async fn prep_async(&self, shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        self.async_flow.prep_async(shared).await
    }
    
    async fn exec_async(&self, _prep_result: &Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        panic!("AsyncParallelBatchFlow cannot exec_async directly. Use run_async() instead.");
    }
    
    async fn post_async(&self, shared: &SharedStore, prep_result: &Box<dyn Any + Send + Sync>,
                      exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        self.async_flow.post_async(shared, prep_result, exec_result).await
    }
    
    async fn run_async(&self, shared: &SharedStore) -> ActionName {
        let prep_result = self.prep_async(shared).await;
        
        // Process batch params in parallel
        if let Some(batch_params) = prep_result.downcast_ref::<Vec<ParamMap>>() {
            let futures = batch_params.iter().map(|params| {
                let params_clone = params.clone();
                let shared_clone = shared.clone();
                
                // Instead of trying to clone AsyncFlow, just create a new one with a placeholder
                // and run directly against the shared store
                async move {
                    // Use shared orchestration logic but avoid direct cloning
                    let action_name = self.async_flow.orchestrate_async(&shared_clone, Some(params_clone)).await;
                    ActionName::default()
                }
            });
            
            // Run all futures in parallel
            future::join_all(futures).await;
        }
        
        self.post_async(shared, &prep_result, Box::new(())).await
    }
} 