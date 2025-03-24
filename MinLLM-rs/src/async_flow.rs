use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::any::Any;
use async_trait::async_trait;
use futures::future;
use serde_json::Value;
use log::warn;

use crate::base::{BaseNode, Node, SharedState, Action};
use crate::flow::{Flow, BatchFlow};
use crate::async_node::AsyncNodeTrait;
use crate::error::{Error, Result};

/// A workflow with asynchronous execution
#[derive(Clone)]
pub struct AsyncFlow {
    /// Underlying flow
    flow: Flow,
    
    /// Base node implementation
    base: BaseNode,
}

impl AsyncFlow {
    /// Create a new async flow with a starting node
    pub fn new(start: Arc<dyn Node>) -> Self {
        Self {
            flow: Flow::new(start),
            base: BaseNode::new(),
        }
    }
    
    /// Check if a node is an async node
    fn is_async(&self, node: &Arc<dyn Node>) -> bool {
        // Try to cast to the trait object, just to check if it's possible
        // We can't use the result directly, we just want to know if it's possible
        let type_id = node.type_id();
        // Check against the type IDs of our async node types
        let async_node_ids = [
            std::any::TypeId::of::<dyn AsyncNodeTrait>(),
            // Add other async node type IDs if needed
        ];
        async_node_ids.contains(&type_id)
    }
    
    /// Orchestrate flow through nodes asynchronously
    pub async fn _orch_async(&self, shared: &mut SharedState, params: Option<HashMap<String, Value>>) -> Result<()> {
        let mut curr = self.flow.start.clone();
        let params = params.unwrap_or_else(|| {
            self.base.params().read().unwrap().clone()
        });
        
        curr.set_params(params);
        
        while let Some(node) = curr.clone().into() {
            let action = if self.is_async(&node) {
                // This is an async node, use dynamic dispatch to call the async method
                // For simplicity, we'll just implement a mock here
                // In a real implementation, you'd need to handle this more robustly
                Err(Error::InvalidOperation("Dynamic dispatch for async nodes not implemented".into()))?
            } else {
                // Not an async node, use the synchronous method
                node._run(shared)?
            };
            
            curr = match self.flow.get_next_node(node, action) {
                Some(next) => next,
                None => break,
            };
        }
        
        Ok(())
    }
}

impl Node for AsyncFlow {
    fn params(&self) -> Arc<RwLock<HashMap<String, Value>>> {
        self.base.params()
    }
    
    fn successors(&self) -> Arc<RwLock<HashMap<String, Arc<dyn Node>>>> {
        self.base.successors()
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
    
    fn prep(&self, _shared: &mut SharedState) -> Result<Value> {
        Err(Error::InvalidOperation("Use prep_async".into()))
    }
    
    fn exec(&self, _prep_res: Value) -> Result<Value> {
        Err(Error::InvalidOperation("AsyncFlow can't exec".into()))
    }
    
    fn post(&self, _shared: &mut SharedState, _prep_res: Value, _exec_res: Value) -> Result<Action> {
        Err(Error::InvalidOperation("Use post_async".into()))
    }
    
    fn _run(&self, _shared: &mut SharedState) -> Result<Action> {
        Err(Error::InvalidOperation("Use run_async".into()))
    }
}

#[async_trait]
impl AsyncNodeTrait for AsyncFlow {
    async fn _exec_async(&self, _prep_res: Value) -> Result<Value> {
        Err(Error::InvalidOperation("AsyncFlow can't exec".into()))
    }
    
    async fn _run_async(&self, shared: &mut SharedState) -> Result<Action> {
        let prep_res = self.prep_async(shared).await?;
        self._orch_async(shared, None).await?;
        self.post_async(shared, prep_res, Value::Null).await
    }
}

/// An async flow that processes batches of items
#[derive(Clone)]
pub struct AsyncBatchFlow {
    /// Underlying async flow
    flow: AsyncFlow,
    
    /// Underlying batch flow
    batch_flow: BatchFlow,
}

impl AsyncBatchFlow {
    /// Create a new async batch flow with a starting node
    pub fn new(start: Arc<dyn Node>) -> Self {
        Self {
            flow: AsyncFlow::new(start.clone()),
            batch_flow: BatchFlow::new(start),
        }
    }
}

impl Node for AsyncBatchFlow {
    fn params(&self) -> Arc<RwLock<HashMap<String, Value>>> {
        self.flow.params()
    }
    
    fn successors(&self) -> Arc<RwLock<HashMap<String, Arc<dyn Node>>>> {
        self.flow.successors()
    }
    
    fn set_params(&self, params: HashMap<String, Value>) {
        self.flow.set_params(params);
    }
    
    fn add_successor(&self, node: Arc<dyn Node>, action: &str) -> Result<Arc<dyn Node>> {
        self.flow.add_successor(node, action)
    }
    
    fn prep(&self, _shared: &mut SharedState) -> Result<Value> {
        Err(Error::InvalidOperation("Use prep_async".into()))
    }
    
    fn exec(&self, _prep_res: Value) -> Result<Value> {
        Err(Error::InvalidOperation("AsyncBatchFlow can't exec".into()))
    }
    
    fn post(&self, _shared: &mut SharedState, _prep_res: Value, _exec_res: Value) -> Result<Action> {
        Err(Error::InvalidOperation("Use post_async".into()))
    }
    
    fn _run(&self, _shared: &mut SharedState) -> Result<Action> {
        Err(Error::InvalidOperation("Use run_async".into()))
    }
}

#[async_trait]
impl AsyncNodeTrait for AsyncBatchFlow {
    async fn _exec_async(&self, _prep_res: Value) -> Result<Value> {
        Err(Error::InvalidOperation("AsyncBatchFlow can't exec".into()))
    }
    
    async fn _run_async(&self, shared: &mut SharedState) -> Result<Action> {
        let prep_res = self.prep_async(shared).await?;
        
        let batch_params = match &prep_res {
            Value::Array(items) => items
                .iter()
                .map(|v| {
                    if let Value::Object(map) = v {
                        let map: HashMap<String, Value> = map
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        Ok(map)
                    } else {
                        Err(Error::NodeExecution("AsyncBatchFlow prep should return array of objects".into()))
                    }
                })
                .collect::<Result<Vec<_>>>()?,
            Value::Null => vec![],
            _ => return Err(Error::NodeExecution("AsyncBatchFlow prep should return array or null".into())),
        };
        
        let flow_params = self.flow.params().read().unwrap().clone();
        
        for mut bp in batch_params {
            // Merge batch params with flow params
            for (k, v) in flow_params.clone() {
                bp.entry(k).or_insert(v);
            }
            
            self.flow._orch_async(shared, Some(bp)).await?;
        }
        
        self.post_async(shared, prep_res, Value::Null).await
    }
}

/// An async flow that processes batches of items in parallel
#[derive(Clone)]
pub struct AsyncParallelBatchFlow {
    /// Underlying async batch flow
    batch_flow: AsyncBatchFlow,
}

impl AsyncParallelBatchFlow {
    /// Create a new async parallel batch flow with a starting node
    pub fn new(start: Arc<dyn Node>) -> Self {
        Self {
            batch_flow: AsyncBatchFlow::new(start),
        }
    }
}

impl Node for AsyncParallelBatchFlow {
    fn params(&self) -> Arc<RwLock<HashMap<String, Value>>> {
        self.batch_flow.params()
    }
    
    fn successors(&self) -> Arc<RwLock<HashMap<String, Arc<dyn Node>>>> {
        self.batch_flow.successors()
    }
    
    fn set_params(&self, params: HashMap<String, Value>) {
        self.batch_flow.set_params(params);
    }
    
    fn add_successor(&self, node: Arc<dyn Node>, action: &str) -> Result<Arc<dyn Node>> {
        self.batch_flow.add_successor(node, action)
    }
    
    fn prep(&self, _shared: &mut SharedState) -> Result<Value> {
        Err(Error::InvalidOperation("Use prep_async".into()))
    }
    
    fn exec(&self, _prep_res: Value) -> Result<Value> {
        Err(Error::InvalidOperation("AsyncParallelBatchFlow can't exec".into()))
    }
    
    fn post(&self, _shared: &mut SharedState, _prep_res: Value, _exec_res: Value) -> Result<Action> {
        Err(Error::InvalidOperation("Use post_async".into()))
    }
    
    fn _run(&self, _shared: &mut SharedState) -> Result<Action> {
        Err(Error::InvalidOperation("Use run_async".into()))
    }
}

#[async_trait]
impl AsyncNodeTrait for AsyncParallelBatchFlow {
    async fn prep_async(&self, shared: &mut SharedState) -> Result<Value> {
        self.batch_flow.prep_async(shared).await
    }
    
    async fn post_async(&self, shared: &mut SharedState, prep_res: Value, exec_res: Value) -> Result<Action> {
        self.batch_flow.post_async(shared, prep_res, exec_res).await
    }
    
    async fn _exec_async(&self, _prep_res: Value) -> Result<Value> {
        Err(Error::InvalidOperation("AsyncParallelBatchFlow can't exec".into()))
    }
    
    async fn _run_async(&self, shared: &mut SharedState) -> Result<Action> {
        let prep_res = self.prep_async(shared).await?;
        
        let batch_params = match &prep_res {
            Value::Array(items) => items
                .iter()
                .map(|v| {
                    if let Value::Object(map) = v {
                        let map: HashMap<String, Value> = map
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        Ok(map)
                    } else {
                        Err(Error::NodeExecution("AsyncParallelBatchFlow prep should return array of objects".into()))
                    }
                })
                .collect::<Result<Vec<_>>>()?,
            Value::Null => vec![],
            _ => return Err(Error::NodeExecution("AsyncParallelBatchFlow prep should return array or null".into())),
        };
        
        if batch_params.is_empty() {
            return self.post_async(shared, prep_res, Value::Null).await;
        }
        
        let flow_params = self.batch_flow.params().read().unwrap().clone();
        
        // Create a future for each batch item
        let futures = batch_params
            .into_iter()
            .map(|mut bp| {
                // Clone what we need for the future
                let flow = self.batch_flow.flow.clone();
                let mut shared_clone = shared.clone();
                let flow_params = flow_params.clone();
                
                // Merge batch params with flow params
                for (k, v) in flow_params {
                    bp.entry(k).or_insert(v);
                }
                
                async move { flow._orch_async(&mut shared_clone, Some(bp)).await }
            })
            .collect::<Vec<_>>();
        
        // Execute all futures concurrently
        let results = future::join_all(futures).await;
        
        // Check for errors
        for result in results {
            result?;
        }
        
        self.post_async(shared, prep_res, Value::Null).await
    }
} 