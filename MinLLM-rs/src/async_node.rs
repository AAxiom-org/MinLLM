use std::any::Any;
use std::collections::HashMap;
use std::time::Duration;
use async_trait::async_trait;
use tokio::time;

use crate::error::{ActionName, MinLLMError, Result};
use crate::node::{Node, BaseNode, RegularNode, BatchNode, ParamMap};
use crate::store::SharedStore;
use crate::flow::AsyncNode;

/// Implementation of an asynchronous node
pub struct AsyncNodeImpl {
    base: BaseNode,
    max_retries: usize,
    wait: u64,
    current_retry: usize,
}

impl AsyncNodeImpl {
    pub fn new(max_retries: usize, wait: u64) -> Self {
        Self {
            base: BaseNode::new(),
            max_retries,
            wait,
            current_retry: 0,
        }
    }
    
    /// Fallback method for when async execution fails after retries
    pub async fn exec_fallback_async(&self, _prep_result: Box<dyn Any + Send + Sync>, 
                                   exc: Box<dyn std::error::Error + Send + Sync>) 
        -> Box<dyn Any + Send + Sync> {
        // Default implementation just re-raises the exception
        panic!("Async node execution failed after {} retries: {:?}", self.max_retries, exc);
    }
}

#[async_trait]
impl Node for AsyncNodeImpl {
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
    
    fn prep(&self, _shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        panic!("Use prep_async instead for AsyncNode");
    }
    
    fn exec(&self, _prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        panic!("Use exec_async instead for AsyncNode");
    }
    
    fn post(&self, _shared: &SharedStore, _prep_result: Box<dyn Any + Send + Sync>, 
           _exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        panic!("Use post_async instead for AsyncNode");
    }
}

#[async_trait]
impl AsyncNode for AsyncNodeImpl {
    async fn prep_async(&self, _shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        Box::new(())
    }
    
    async fn exec_async(&self, _prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        Box::new(())
    }
    
    async fn post_async(&self, _shared: &SharedStore, _prep_result: Box<dyn Any + Send + Sync>,
                      _exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        ActionName::default()
    }
    
    async fn run_async(&self, shared: &SharedStore) -> ActionName {
        if !self.base.successors.is_empty() {
            eprintln!("Warning: Node won't run successors. Use AsyncFlow.");
        }
        
        let prep_result = self.prep_async(shared).await;
        let mut retry_count = 0;
        
        loop {
            match tokio::task::spawn_blocking(move || {
                // This would be a future that can fail in a real implementation
                // For now, we just return successful execution
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(prep_result.clone())
            }).await.unwrap() {
                Ok(result) => {
                    let exec_result = self.exec_async(result.clone()).await;
                    return self.post_async(shared, result, exec_result).await;
                },
                Err(err) => {
                    retry_count += 1;
                    if retry_count >= self.max_retries {
                        let exec_result = self.exec_fallback_async(prep_result.clone(), err).await;
                        return self.post_async(shared, prep_result.clone(), exec_result).await;
                    }
                    
                    if self.wait > 0 {
                        time::sleep(Duration::from_millis(self.wait)).await;
                    }
                }
            }
        }
    }
}

/// AsyncBatchNode processes batches of items asynchronously
pub struct AsyncBatchNode {
    async_node: AsyncNodeImpl,
}

impl AsyncBatchNode {
    pub fn new(max_retries: usize, wait: u64) -> Self {
        Self {
            async_node: AsyncNodeImpl::new(max_retries, wait),
        }
    }
}

#[async_trait]
impl Node for AsyncBatchNode {
    fn set_params(&mut self, params: ParamMap) {
        self.async_node.set_params(params);
    }
    
    fn add_successor(&mut self, node: Box<dyn Node>, action: impl Into<ActionName>) -> &mut Self {
        self.async_node.add_successor(node, action);
        self
    }
    
    fn get_successor(&self, action: &str) -> Option<&Box<dyn Node>> {
        self.async_node.get_successor(action)
    }
    
    fn prep(&self, _shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        panic!("Use prep_async instead for AsyncBatchNode");
    }
    
    fn exec(&self, _prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        panic!("Use exec_async instead for AsyncBatchNode");
    }
    
    fn post(&self, _shared: &SharedStore, _prep_result: Box<dyn Any + Send + Sync>, 
           _exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        panic!("Use post_async instead for AsyncBatchNode");
    }
}

#[async_trait]
impl AsyncNode for AsyncBatchNode {
    async fn prep_async(&self, shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        self.async_node.prep_async(shared).await
    }
    
    async fn exec_async(&self, prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        self.async_node.exec_async(prep_result).await
    }
    
    async fn post_async(&self, shared: &SharedStore, prep_result: Box<dyn Any + Send + Sync>,
                      exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        self.async_node.post_async(shared, prep_result, exec_result).await
    }
    
    async fn run_async(&self, shared: &SharedStore) -> ActionName {
        if !self.async_node.base.successors.is_empty() {
            eprintln!("Warning: Node won't run successors. Use AsyncFlow.");
        }
        
        let prep_result = self.prep_async(shared).await;
        
        // Process batch items
        if let Some(items) = prep_result.downcast_ref::<Vec<Box<dyn Any + Send + Sync>>>() {
            let mut results = Vec::with_capacity(items.len());
            
            for item in items {
                let result = self.async_node.run_async(shared).await;
                results.push(Box::new(result) as Box<dyn Any + Send + Sync>);
            }
            
            let exec_result = Box::new(results);
            return self.post_async(shared, prep_result, exec_result).await;
        }
        
        // If not a batch, process as a single item
        self.async_node.run_async(shared).await
    }
}

/// AsyncParallelBatchNode processes batches of items asynchronously in parallel
pub struct AsyncParallelBatchNode {
    async_node: AsyncNodeImpl,
}

impl AsyncParallelBatchNode {
    pub fn new(max_retries: usize, wait: u64) -> Self {
        Self {
            async_node: AsyncNodeImpl::new(max_retries, wait),
        }
    }
}

#[async_trait]
impl Node for AsyncParallelBatchNode {
    fn set_params(&mut self, params: ParamMap) {
        self.async_node.set_params(params);
    }
    
    fn add_successor(&mut self, node: Box<dyn Node>, action: impl Into<ActionName>) -> &mut Self {
        self.async_node.add_successor(node, action);
        self
    }
    
    fn get_successor(&self, action: &str) -> Option<&Box<dyn Node>> {
        self.async_node.get_successor(action)
    }
    
    fn prep(&self, _shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        panic!("Use prep_async instead for AsyncParallelBatchNode");
    }
    
    fn exec(&self, _prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        panic!("Use exec_async instead for AsyncParallelBatchNode");
    }
    
    fn post(&self, _shared: &SharedStore, _prep_result: Box<dyn Any + Send + Sync>, 
           _exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        panic!("Use post_async instead for AsyncParallelBatchNode");
    }
}

#[async_trait]
impl AsyncNode for AsyncParallelBatchNode {
    async fn prep_async(&self, shared: &SharedStore) -> Box<dyn Any + Send + Sync> {
        self.async_node.prep_async(shared).await
    }
    
    async fn exec_async(&self, prep_result: Box<dyn Any + Send + Sync>) -> Box<dyn Any + Send + Sync> {
        self.async_node.exec_async(prep_result).await
    }
    
    async fn post_async(&self, shared: &SharedStore, prep_result: Box<dyn Any + Send + Sync>,
                      exec_result: Box<dyn Any + Send + Sync>) -> ActionName {
        self.async_node.post_async(shared, prep_result, exec_result).await
    }
    
    async fn run_async(&self, shared: &SharedStore) -> ActionName {
        if !self.async_node.base.successors.is_empty() {
            eprintln!("Warning: Node won't run successors. Use AsyncFlow.");
        }
        
        let prep_result = self.prep_async(shared).await;
        
        // Process batch items in parallel
        if let Some(items) = prep_result.downcast_ref::<Vec<Box<dyn Any + Send + Sync>>>() {
            let futures = items.iter().map(|item| {
                let item_clone = item.clone();
                let shared_clone = shared.clone();
                let self_clone = self.async_node.clone();
                
                async move {
                    self_clone.run_async(&shared_clone).await
                }
            });
            
            let results = futures::future::join_all(futures).await;
            let boxed_results: Vec<Box<dyn Any + Send + Sync>> = results
                .into_iter()
                .map(|r| Box::new(r) as Box<dyn Any + Send + Sync>)
                .collect();
            
            let exec_result = Box::new(boxed_results);
            return self.post_async(shared, prep_result, exec_result).await;
        }
        
        // If not a batch, process as a single item
        self.async_node.run_async(shared).await
    }
} 