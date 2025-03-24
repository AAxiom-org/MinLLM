use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use async_trait::async_trait;
use futures::future::{self};
use tokio::time::sleep;
use serde_json::Value;
use log::warn;

use crate::base::{BaseNode, Node as NodeTrait, SharedState, Action};
use crate::error::{Error, Result};

/// Trait for asynchronous node operations
#[async_trait]
pub trait AsyncNodeTrait: NodeTrait {
    /// Asynchronous preparation step before execution
    async fn prep_async(&self, _shared: &mut SharedState) -> Result<Value> {
        Ok(Value::Null)
    }
    
    /// Asynchronous execution of node logic
    async fn exec_async(&self, _prep_res: Value) -> Result<Value> {
        Ok(Value::Null)
    }
    
    /// Asynchronous post-execution step
    async fn post_async(&self, _shared: &mut SharedState, _prep_res: Value, _exec_res: Value) -> Result<Action> {
        Ok(None)
    }
    
    /// Asynchronous fallback for execution failures
    async fn exec_fallback_async(&self, _prep_res: Value, error: Error) -> Result<Value> {
        Err(error)
    }
    
    /// Internal asynchronous execution method
    async fn _exec_async(&self, prep_res: Value) -> Result<Value>;
    
    /// Run the node asynchronously
    async fn _run_async(&self, shared: &mut SharedState) -> Result<Action> {
        let prep_res = self.prep_async(shared).await?;
        let exec_res = self._exec_async(prep_res.clone()).await?;
        self.post_async(shared, prep_res, exec_res).await
    }
    
    /// Run the node as a standalone (warns if there are successors)
    async fn run_async(&self, shared: &mut SharedState) -> Result<Action> {
        {
            let successors_lock = self.successors();
            let successors = successors_lock.read().unwrap();
            if !successors.is_empty() {
                warn!("AsyncNode won't run successors. Use AsyncFlow.");
            }
        }
        self._run_async(shared).await
    }
}

/// A node with asynchronous execution
#[derive(Clone)]
pub struct AsyncNode {
    /// Base node implementation
    base: BaseNode,
    
    /// Maximum number of retries
    max_retries: usize,
    
    /// Wait time between retries in milliseconds
    wait: u64,
    
    /// Current retry count
    cur_retry: Arc<RwLock<usize>>,
}

impl AsyncNode {
    /// Create a new async node with retry capability
    pub fn new(max_retries: usize, wait: u64) -> Self {
        Self {
            base: BaseNode::new(),
            max_retries,
            wait,
            cur_retry: Arc::new(RwLock::new(0)),
        }
    }
}

impl Default for AsyncNode {
    fn default() -> Self {
        Self::new(1, 0)
    }
}

impl NodeTrait for AsyncNode {
    fn params(&self) -> Arc<RwLock<HashMap<String, Value>>> {
        self.base.params()
    }
    
    fn successors(&self) -> Arc<RwLock<HashMap<String, Arc<dyn NodeTrait>>>> {
        self.base.successors()
    }
    
    fn prep(&self, _shared: &mut SharedState) -> Result<Value> {
        Err(Error::InvalidOperation("Use prep_async".into()))
    }
    
    fn exec(&self, _prep_res: Value) -> Result<Value> {
        Err(Error::InvalidOperation("Use exec_async".into()))
    }
    
    fn post(&self, _shared: &mut SharedState, _prep_res: Value, _exec_res: Value) -> Result<Action> {
        Err(Error::InvalidOperation("Use post_async".into()))
    }
    
    fn _run(&self, _shared: &mut SharedState) -> Result<Action> {
        Err(Error::InvalidOperation("Use run_async".into()))
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
}

#[async_trait]
impl AsyncNodeTrait for AsyncNode {
    async fn _exec_async(&self, prep_res: Value) -> Result<Value> {
        for retry in 0..self.max_retries {
            {
                let mut cur_retry = self.cur_retry.write().unwrap();
                *cur_retry = retry;
            }
            
            match self.exec_async(prep_res.clone()).await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    if retry == self.max_retries - 1 {
                        return self.exec_fallback_async(prep_res, e).await;
                    }
                    
                    if self.wait > 0 {
                        sleep(Duration::from_millis(self.wait)).await;
                    }
                }
            }
        }
        
        // This should never happen if max_retries > 0
        Err(Error::NodeExecution("Max retries exceeded".into()))
    }
}

/// An async node that processes batches of items
#[derive(Clone)]
pub struct AsyncBatchNode {
    /// Underlying async node
    node: AsyncNode,
}

impl AsyncBatchNode {
    /// Create a new async batch node
    pub fn new(max_retries: usize, wait: u64) -> Self {
        Self {
            node: AsyncNode::new(max_retries, wait),
        }
    }
}

impl Default for AsyncBatchNode {
    fn default() -> Self {
        Self::new(1, 0)
    }
}

impl NodeTrait for AsyncBatchNode {
    fn params(&self) -> Arc<RwLock<HashMap<String, Value>>> {
        self.node.params()
    }
    
    fn successors(&self) -> Arc<RwLock<HashMap<String, Arc<dyn NodeTrait>>>> {
        self.node.successors()
    }
    
    fn prep(&self, _shared: &mut SharedState) -> Result<Value> {
        Err(Error::InvalidOperation("Use prep_async".into()))
    }
    
    fn exec(&self, _prep_res: Value) -> Result<Value> {
        Err(Error::InvalidOperation("Use exec_async".into()))
    }
    
    fn post(&self, _shared: &mut SharedState, _prep_res: Value, _exec_res: Value) -> Result<Action> {
        Err(Error::InvalidOperation("Use post_async".into()))
    }
    
    fn _run(&self, _shared: &mut SharedState) -> Result<Action> {
        Err(Error::InvalidOperation("Use run_async".into()))
    }
    
    fn set_params(&self, params: HashMap<String, Value>) {
        self.node.set_params(params);
    }
    
    fn add_successor(&self, node: Arc<dyn NodeTrait>, action: &str) -> Result<Arc<dyn NodeTrait>> {
        self.node.add_successor(node, action)
    }
}

#[async_trait]
impl AsyncNodeTrait for AsyncBatchNode {
    async fn prep_async(&self, shared: &mut SharedState) -> Result<Value> {
        self.node.prep_async(shared).await
    }
    
    async fn exec_async(&self, prep_res: Value) -> Result<Value> {
        self.node.exec_async(prep_res).await
    }
    
    async fn post_async(&self, shared: &mut SharedState, prep_res: Value, exec_res: Value) -> Result<Action> {
        self.node.post_async(shared, prep_res, exec_res).await
    }
    
    async fn exec_fallback_async(&self, prep_res: Value, error: Error) -> Result<Value> {
        self.node.exec_fallback_async(prep_res, error).await
    }
    
    async fn _exec_async(&self, items: Value) -> Result<Value> {
        // Handle empty batches
        if items.is_null() {
            return Ok(Value::Array(vec![]));
        }
        
        // Ensure we have an array
        let items = match items {
            Value::Array(items) => items,
            _ => return Err(Error::NodeExecution("AsyncBatchNode requires an array".into())),
        };
        
        // Process each item sequentially
        let mut results = Vec::with_capacity(items.len());
        for item in items {
            let result = self.node._exec_async(item).await?;
            results.push(result);
        }
        
        Ok(Value::Array(results))
    }
}

/// An async node that processes batches of items in parallel
#[derive(Clone)]
pub struct AsyncParallelBatchNode {
    /// Underlying async node
    node: AsyncNode,
}

impl AsyncParallelBatchNode {
    /// Create a new async parallel batch node
    pub fn new(max_retries: usize, wait: u64) -> Self {
        Self {
            node: AsyncNode::new(max_retries, wait),
        }
    }
}

impl Default for AsyncParallelBatchNode {
    fn default() -> Self {
        Self::new(1, 0)
    }
}

impl NodeTrait for AsyncParallelBatchNode {
    fn params(&self) -> Arc<RwLock<HashMap<String, Value>>> {
        self.node.params()
    }
    
    fn successors(&self) -> Arc<RwLock<HashMap<String, Arc<dyn NodeTrait>>>> {
        self.node.successors()
    }
    
    fn prep(&self, _shared: &mut SharedState) -> Result<Value> {
        Err(Error::InvalidOperation("Use prep_async".into()))
    }
    
    fn exec(&self, _prep_res: Value) -> Result<Value> {
        Err(Error::InvalidOperation("Use exec_async".into()))
    }
    
    fn post(&self, _shared: &mut SharedState, _prep_res: Value, _exec_res: Value) -> Result<Action> {
        Err(Error::InvalidOperation("Use post_async".into()))
    }
    
    fn _run(&self, _shared: &mut SharedState) -> Result<Action> {
        Err(Error::InvalidOperation("Use run_async".into()))
    }
    
    fn set_params(&self, params: HashMap<String, Value>) {
        self.node.set_params(params);
    }
    
    fn add_successor(&self, node: Arc<dyn NodeTrait>, action: &str) -> Result<Arc<dyn NodeTrait>> {
        self.node.add_successor(node, action)
    }
}

#[async_trait]
impl AsyncNodeTrait for AsyncParallelBatchNode {
    async fn prep_async(&self, shared: &mut SharedState) -> Result<Value> {
        self.node.prep_async(shared).await
    }
    
    async fn exec_async(&self, prep_res: Value) -> Result<Value> {
        self.node.exec_async(prep_res).await
    }
    
    async fn post_async(&self, shared: &mut SharedState, prep_res: Value, exec_res: Value) -> Result<Action> {
        self.node.post_async(shared, prep_res, exec_res).await
    }
    
    async fn exec_fallback_async(&self, prep_res: Value, error: Error) -> Result<Value> {
        self.node.exec_fallback_async(prep_res, error).await
    }
    
    async fn _exec_async(&self, items: Value) -> Result<Value> {
        // Handle empty batches
        if items.is_null() {
            return Ok(Value::Array(vec![]));
        }
        
        // Ensure we have an array
        let items = match items {
            Value::Array(items) => items,
            _ => return Err(Error::NodeExecution("AsyncParallelBatchNode requires an array".into())),
        };
        
        // Process all items in parallel
        let futures = items
            .into_iter()
            .map(|item| {
                let node = self.node.clone();
                async move { node._exec_async(item).await }
            })
            .collect::<Vec<_>>();
        
        let results = future::join_all(futures)
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        
        Ok(Value::Array(results))
    }
} 