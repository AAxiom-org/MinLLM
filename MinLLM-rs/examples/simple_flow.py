#!/usr/bin/env python3
"""
A simple example demonstrating the usage of MinLLM.
"""
import asyncio
from minllm import Node, Flow, AsyncNode, AsyncFlow

# Synchronous example
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

# Asynchronous example
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

def run_sync_example():
    print("\n=== Running Synchronous Example ===")
    # Create nodes and flow
    first = FirstNode()
    second = SecondNode()
    flow = Flow(first)
    
    # Connect nodes
    first >> second
    
    # Run flow
    shared_data = {"initial_data": "hello world"}
    flow.run(shared_data)
    print(f"Final result: {shared_data['final_result']}")

async def run_async_example():
    print("\n=== Running Asynchronous Example ===")
    # Create nodes and flow
    first = AsyncFirstNode()
    second = AsyncSecondNode()
    flow = AsyncFlow(first)
    
    # Connect nodes
    first >> second
    
    # Run flow
    shared_data = {"initial_data": "hello async world"}
    await flow.run_async(shared_data)
    print(f"Final result: {shared_data['final_result']}")

if __name__ == "__main__":
    # Run sync example
    run_sync_example()
    
    # Run async example
    asyncio.run(run_async_example()) 