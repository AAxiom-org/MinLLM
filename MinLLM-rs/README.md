# MinLLM-rs

A Rust implementation of MinLLM with Python bindings, providing a fast, efficient framework for building LLM-based applications.

## Overview

MinLLM is a minimalist framework for building complex applications using large language models. This implementation rebuilds the original Python package in Rust, providing significant performance improvements while maintaining the exact same Python API.

Key features:
- **Identical Python API** - Drop-in replacement for the original MinLLM package
- **Performance Improvements** - Significantly faster execution for CPU-bound tasks
- **True Parallelism** - Overcome Python's GIL limitations
- **Type Safety** - Catch errors at compile time rather than runtime
- **Memory Safety** - Without garbage collection overhead

## Installation

```bash
pip install minllm
```

## Usage

The API is identical to the original MinLLM Python package:

```python
from minllm import Node, Flow, SharedStore

class InputNode(Node):
    def prep(self, shared):
        return "Hello World!"
        
    def exec(self, prep_res):
        return prep_res.upper()
        
    def post(self, shared, prep_res, exec_res):
        shared.set("result", exec_res)
        return "default"

class OutputNode(Node):
    def prep(self, shared):
        return shared.get("result")
        
    def exec(self, prep_res):
        print(f"Result: {prep_res}")
        return prep_res
        
    def post(self, shared, prep_res, exec_res):
        return "default"

# Create and run a flow
flow = Flow(InputNode() >> OutputNode())
shared = SharedStore()
flow.run(shared)
```

### Async Support

```python
import asyncio
from minllm import AsyncNode, AsyncFlow, SharedStore

class AsyncInputNode(AsyncNode):
    async def prep_async(self, shared):
        await asyncio.sleep(1)  # Async operation
        return "Hello World!"
        
    async def exec_async(self, prep_res):
        await asyncio.sleep(1)  # Async operation
        return prep_res.upper()
        
    async def post_async(self, shared, prep_res, exec_res):
        shared.set("result", exec_res)
        return "default"

# Create and run an async flow
flow = AsyncFlow(AsyncInputNode() >> AsyncOutputNode())
shared = SharedStore()
asyncio.run(flow.run_async(shared))
```

### Batch Processing

```python
from minllm import BatchNode, BatchFlow, SharedStore

class InputBatchNode(BatchNode):
    def prep(self, shared):
        # Return a list of parameter dicts to be processed
        return [{'item': 'apple'}, {'item': 'banana'}, {'item': 'grape'}]

# Create and run a batch flow
flow = BatchFlow(InputBatchNode() >> ProcessNode())
shared = SharedStore()
flow.run(shared)
```

## Core Components

- **Node**: Basic building block for computation
- **Flow**: Orchestrates the execution of nodes
- **SharedStore**: Shared key-value store for communication between nodes
- **BatchNode/BatchFlow**: Process batches of items
- **AsyncNode/AsyncFlow**: Support for asynchronous operations

## Examples

See the `examples/python` directory for more detailed examples.

## Building from Source

To build the package from source, you'll need Rust and Python development tools:

```bash
# Clone the repository
git clone https://github.com/username/MinLLM-rs
cd MinLLM-rs

# Build the package
maturin develop

# Run tests
cargo test
```

## License

MIT 