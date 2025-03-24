use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde_json::Value;
use log::warn;

use crate::base::{BaseNode, Node, SharedState, Action};
use crate::error::{Error, Result};

/// A workflow that orchestrates execution through nodes
#[derive(Clone)]
pub struct Flow {
    /// Base node implementation
    base: BaseNode,
    
    /// The starting node of the flow
    pub start: Arc<dyn Node>,
}

impl Flow {
    /// Create a new flow with a starting node
    pub fn new(start: Arc<dyn Node>) -> Self {
        Self {
            base: BaseNode::new(),
            start,
        }
    }
    
    /// Get the next node based on the current node and action
    pub fn get_next_node(&self, curr: Arc<dyn Node>, action: Action) -> Option<Arc<dyn Node>> {
        let action_key = action.unwrap_or_else(|| "default".to_string());
        let successors_lock = curr.successors();
        let successors = successors_lock.read().unwrap();
        
        let next = successors.get(&action_key).cloned();
        
        if next.is_none() && !successors.is_empty() {
            let actions: Vec<String> = successors.keys().cloned().collect();
            warn!("Flow ends: '{}' not found in {:?}", action_key, actions);
        }
        
        next
    }
    
    /// Orchestrate flow through nodes
    pub fn _orch(&self, shared: &mut SharedState, params: Option<HashMap<String, Value>>) -> Result<()> {
        let mut curr = self.start.clone();
        let params = params.unwrap_or_else(|| {
            self.base.params().read().unwrap().clone()
        });
        
        curr.set_params(params);
        
        while let Some(node) = curr.clone().into() {
            let action = node._run(shared)?;
            curr = match self.get_next_node(node, action) {
                Some(next) => next,
                None => break,
            };
        }
        
        Ok(())
    }
}

impl Node for Flow {
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
    
    fn _run(&self, shared: &mut SharedState) -> Result<Action> {
        let prep_res = self.prep(shared)?;
        self._orch(shared, None)?;
        self.post(shared, prep_res, Value::Null)
    }
    
    fn exec(&self, _prep_res: Value) -> Result<Value> {
        Err(Error::InvalidOperation("Flow can't exec.".into()))
    }
}

/// A flow that processes batches of items
#[derive(Clone)]
pub struct BatchFlow {
    /// The underlying flow
    flow: Flow,
}

impl BatchFlow {
    /// Create a new batch flow with a starting node
    pub fn new(start: Arc<dyn Node>) -> Self {
        Self {
            flow: Flow::new(start),
        }
    }
}

impl Node for BatchFlow {
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
    
    fn _run(&self, shared: &mut SharedState) -> Result<Action> {
        let prep_res = self.prep(shared)?;
        
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
                        Err(Error::NodeExecution("BatchFlow prep should return array of objects".into()))
                    }
                })
                .collect::<Result<Vec<_>>>()?,
            Value::Null => vec![],
            _ => return Err(Error::NodeExecution("BatchFlow prep should return array or null".into())),
        };
        
        let flow_params = self.flow.params().read().unwrap().clone();
        
        for mut bp in batch_params {
            // Merge batch params with flow params
            for (k, v) in flow_params.clone() {
                bp.entry(k).or_insert(v);
            }
            
            self.flow._orch(shared, Some(bp))?;
        }
        
        self.post(shared, prep_res, Value::Null)
    }
    
    fn exec(&self, _prep_res: Value) -> Result<Value> {
        Err(Error::InvalidOperation("BatchFlow can't exec.".into()))
    }
} 