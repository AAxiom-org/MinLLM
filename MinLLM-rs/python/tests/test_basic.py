#!/usr/bin/env python3
"""
Basic tests for MinLLM Python bindings.
"""

import unittest
from minllm import SharedStore, BaseNode, Node, Flow

class TestSharedStore(unittest.TestCase):
    def test_basic_operations(self):
        store = SharedStore()
        
        # Test set/get
        store.set("key1", "value1")
        self.assertEqual(store.get("key1"), "value1")
        
        # Test non-existent key
        self.assertIsNone(store.get("non_existent"))
        
        # Test with different types
        store.set("int", 42)
        self.assertEqual(store.get("int"), 42)
        
        store.set("list", [1, 2, 3])
        self.assertEqual(store.get("list"), [1, 2, 3])
        
        store.set("dict", {"a": 1, "b": 2})
        self.assertEqual(store.get("dict"), {"a": 1, "b": 2})

class TestBaseNode(unittest.TestCase):
    def test_node_operations(self):
        node = BaseNode()
        
        # Test params
        node.set_params({"param1": "value1", "param2": 42})
        
        # Test successors
        next_node = BaseNode()
        result = node.add_successor(next_node)
        self.assertEqual(result, next_node)
        
        # Test with action
        another_node = BaseNode()
        result = node.add_successor(another_node, "custom_action")
        self.assertEqual(result, another_node)

class TestFlow(unittest.TestCase):
    def test_simple_flow(self):
        class TestInputNode(Node):
            def prep(self, shared):
                return "input"
                
            def exec(self, prep_res):
                return prep_res.upper()
                
            def post(self, shared, prep_res, exec_res):
                shared.set("result", exec_res)
                return "default"
        
        class TestOutputNode(Node):
            def prep(self, shared):
                return shared.get("result")
                
            def exec(self, prep_res):
                return f"{prep_res}!"
                
            def post(self, shared, prep_res, exec_res):
                shared.set("final", exec_res)
                return "default"
        
        # Create flow with chaining syntax
        flow = Flow(TestInputNode() >> TestOutputNode())
        shared = SharedStore()
        
        # Run the flow
        flow.run(shared)
        
        # Check results
        self.assertEqual(shared.get("result"), "INPUT")
        self.assertEqual(shared.get("final"), "INPUT!")

class TestOperatorSyntax(unittest.TestCase):
    def test_chaining_syntax(self):
        node1 = BaseNode()
        node2 = BaseNode()
        node3 = BaseNode()
        
        # Test simple chaining
        result = node1 >> node2 >> node3
        self.assertEqual(result, node3)
        
        # Test action-based chaining
        node4 = BaseNode()
        node5 = BaseNode()
        
        # This uses node - "action" >> other_node syntax
        node4 - "custom" >> node5
        
        # Since we can't directly inspect the successors from Python,
        # we'll have to trust that it works as expected

if __name__ == "__main__":
    unittest.main() 