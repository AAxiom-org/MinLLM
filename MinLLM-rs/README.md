# MinLLM

A workflow orchestration library implemented in Rust with Python bindings.

This library provides tools for building and executing workflow pipelines with synchronous and asynchronous execution patterns.

## Features

- Build workflow pipelines using composable nodes
- Synchronous and asynchronous execution
- Batch processing
- Parallel execution for batch operations
- Error handling with retry logic
- Conditional transitions between nodes

## Installation

### From PyPI

```bash
pip install minllm
```

### From Source

```bash
git clone https://github.com/yourusername/minllm
cd minllm
pip install maturin
maturin develop
```

## Usage

### Basic Example

```python
from minllm import Node, Flow

# Define nodes
class FirstNode(Node):
    def prep(self, shared):
        print("FirstNode: prep")
        return {"data": shared.get("initial_data", "default")}
    
    def exec(self, prep_res):
        print(f"FirstNode: exec with {prep_res}")
        return prep_res["data"].upper()
    
    def post(self, shared, prep_res, exec_res):
        print(f"FirstNode: post with result {exec_res}")
        shared["first_result"] = exec_res
        return "default"  # action to determine next node

class SecondNode(Node):
    def prep(self, shared):
        print("SecondNode: prep")
        return {"prev_result": shared.get("first_result", "")}
    
    def exec(self, prep_res):
        print(f"SecondNode: exec with {prep_res}")
        return f"Processed: {prep_res['prev_result']}"
    
    def post(self, shared, prep_res, exec_res):
        print(f"SecondNode: post with result {exec_res}")
        shared["final_result"] = exec_res
        return None  # end of flow

# Create flow
first = FirstNode()
second = SecondNode()
flow = Flow(first)

# Connect nodes
first >> second

# Run flow
shared_data = {"initial_data": "hello world"}
flow.run(shared_data)
print(f"Final result: {shared_data['final_result']}")
```

### Asynchronous Example

```python
import asyncio
from minllm import AsyncNode, AsyncFlow

class AsyncFirstNode(AsyncNode):
    async def prep_async(self, shared):
        print("AsyncFirstNode: prep")
        return {"data": shared.get("initial_data", "default")}
    
    async def exec_async(self, prep_res):
        print(f"AsyncFirstNode: exec with {prep_res}")
        await asyncio.sleep(0.1)  # Simulate async work
        return prep_res["data"].upper()
    
    async def post_async(self, shared, prep_res, exec_res):
        print(f"AsyncFirstNode: post with result {exec_res}")
        shared["first_result"] = exec_res
        return "default"  # action to determine next node

class AsyncSecondNode(AsyncNode):
    async def prep_async(self, shared):
        print("AsyncSecondNode: prep")
        return {"prev_result": shared.get("first_result", "")}
    
    async def exec_async(self, prep_res):
        print(f"AsyncSecondNode: exec with {prep_res}")
        await asyncio.sleep(0.1)  # Simulate async work
        return f"Processed: {prep_res['prev_result']}"
    
    async def post_async(self, shared, prep_res, exec_res):
        print(f"AsyncSecondNode: post with result {exec_res}")
        shared["final_result"] = exec_res
        return None  # end of flow

# Create flow
first = AsyncFirstNode()
second = AsyncSecondNode()
flow = AsyncFlow(first)

# Connect nodes
first >> second

# Run flow
async def main():
    shared_data = {"initial_data": "hello async world"}
    await flow.run_async(shared_data)
    print(f"Final result: {shared_data['final_result']}")

asyncio.run(main())
```

### Conditional Transitions

```python
# Connect nodes with conditional transitions
first - "success" >> success_node
first - "error" >> error_node
```

### Batch Processing

```python
from minllm import BatchNode, BatchFlow

class BatchProcessor(BatchNode):
    def exec(self, items):
        return [item.upper() for item in items]

# Create batch flow
batch_node = BatchProcessor()
flow = BatchFlow(batch_node)

# Run batch flow
shared_data = {"items": ["item1", "item2", "item3"]}
flow.run(shared_data)
```

## Rust Usage

This library can also be used directly from Rust:

```rust
use minllm::{BaseNode, Node, Flow};
use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;

struct MyNode {
    node: Node,
}

impl MyNode {
    fn new() -> Self {
        Self {
            node: Node::new(1, 0),
        }
    }
}

impl RustNodeTrait for MyNode {
    // Implement required trait methods
}

fn main() {
    let first = Arc::new(MyNode::new());
    let second = Arc::new(MyNode::new());
    
    let flow = Flow::new(first.clone());
    first.add_successor(second, "default").unwrap();
    
    let mut shared = HashMap::new();
    shared.insert("initial_data".to_string(), Value::String("hello".to_string()));
    
    flow.run(&mut shared).unwrap();
}
```

## License

MIT License 