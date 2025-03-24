#!/usr/bin/env python3
"""
Async tests for MinLLM Python bindings.
"""

import unittest
import asyncio
from minllm import SharedStore, AsyncNode, AsyncFlow, AsyncBatchNode, AsyncBatchFlow

class TestAsyncFlow(unittest.IsolatedAsyncioTestCase):
    async def test_simple_async_flow(self):
        class TestAsyncInputNode(AsyncNode):
            async def prep_async(self, shared):
                return "input"
                
            async def exec_async(self, prep_res):
                return prep_res.upper()
                
            async def post_async(self, shared, prep_res, exec_res):
                shared.set("result", exec_res)
                return "default"
        
        class TestAsyncOutputNode(AsyncNode):
            async def prep_async(self, shared):
                return shared.get("result")
                
            async def exec_async(self, prep_res):
                return f"{prep_res}!"
                
            async def post_async(self, shared, prep_res, exec_res):
                shared.set("final", exec_res)
                return "default"
        
        # Create flow with chaining syntax
        flow = AsyncFlow(TestAsyncInputNode() >> TestAsyncOutputNode())
        shared = SharedStore()
        
        # Run the flow
        await flow.run_async(shared)
        
        # Check results
        self.assertEqual(shared.get("result"), "INPUT")
        self.assertEqual(shared.get("final"), "INPUT!")

class TestAsyncBatchFlow(unittest.IsolatedAsyncioTestCase):
    async def test_async_batch_flow(self):
        class TestAsyncBatchNode(AsyncBatchNode):
            async def prep_async(self, shared):
                return [
                    {"item": "apple"},
                    {"item": "banana"},
                    {"item": "grape"}
                ]
        
        class TestAsyncProcessNode(AsyncNode):
            async def prep_async(self, shared):
                return self.params
                
            async def exec_async(self, params):
                return f"Processed {params['item']}"
                
            async def post_async(self, shared, prep_res, exec_res):
                results = shared.get("results") or []
                results.append(exec_res)
                shared.set("results", results)
                return "default"
        
        # Create flow with chaining syntax
        flow = AsyncBatchFlow(TestAsyncBatchNode() >> TestAsyncProcessNode())
        shared = SharedStore()
        
        # Run the flow
        await flow.run_async(shared)
        
        # Check results
        results = shared.get("results")
        self.assertIsNotNone(results)
        self.assertEqual(len(results), 3)
        self.assertIn("Processed apple", results)
        self.assertIn("Processed banana", results)
        self.assertIn("Processed grape", results)

if __name__ == "__main__":
    unittest.main() 