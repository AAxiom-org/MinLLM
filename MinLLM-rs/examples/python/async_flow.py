#!/usr/bin/env python3
"""
Async example demonstrating the MinLLM package.
This replicates the exact same API as the original Python version.
"""

import asyncio
from minllm import AsyncNode, AsyncFlow, SharedStore

class AsyncInputNode(AsyncNode):
    async def prep_async(self, shared):
        print("AsyncInputNode: Reading input data")
        await asyncio.sleep(0.5)  # Simulate async I/O
        return "Hello World!"
        
    async def exec_async(self, prep_res):
        print(f"AsyncInputNode: Processing '{prep_res}'")
        await asyncio.sleep(0.5)  # Simulate async computation
        return prep_res.upper()
        
    async def post_async(self, shared, prep_res, exec_res):
        print(f"AsyncInputNode: Saving result '{exec_res}'")
        shared.set("uppercase", exec_res)
        return "default"

class AsyncProcessNode(AsyncNode):
    async def prep_async(self, shared):
        print("AsyncProcessNode: Getting uppercase text")
        await asyncio.sleep(0.3)  # Simulate async I/O
        return shared.get("uppercase")
        
    async def exec_async(self, prep_res):
        print(f"AsyncProcessNode: Adding exclamation to '{prep_res}'")
        await asyncio.sleep(0.3)  # Simulate async computation
        return f"{prep_res}!!!"
        
    async def post_async(self, shared, prep_res, exec_res):
        print(f"AsyncProcessNode: Saving result '{exec_res}'")
        shared.set("result", exec_res)
        return "default"

class AsyncOutputNode(AsyncNode):
    async def prep_async(self, shared):
        print("AsyncOutputNode: Getting result")
        await asyncio.sleep(0.2)  # Simulate async I/O
        return shared.get("result")
        
    async def exec_async(self, prep_res):
        print(f"AsyncOutputNode: Final result: '{prep_res}'")
        await asyncio.sleep(0.2)  # Simulate async computation
        return prep_res
        
    async def post_async(self, shared, prep_res, exec_res):
        print("AsyncOutputNode: Done")
        return "default"

async def main():
    print("Creating async flow with AsyncInputNode >> AsyncProcessNode >> AsyncOutputNode")
    
    # Create a flow using the same syntax as the original Python version
    flow = AsyncFlow(
        AsyncInputNode() >> AsyncProcessNode() >> AsyncOutputNode()
    )
    
    # Create a shared store
    shared = SharedStore()
    
    # Run the flow
    print("\nRunning async flow...")
    await flow.run_async(shared)
    
    # Check the result
    print("\nFinal result in store:", shared.get("result"))

if __name__ == "__main__":
    asyncio.run(main()) 