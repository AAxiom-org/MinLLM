#!/usr/bin/env python3
"""
Simple example demonstrating the MinLLM package.
This replicates the exact same API as the original Python version.
"""

from minllm import Node, Flow, SharedStore

class InputNode(Node):
    def prep(self, shared):
        print("InputNode: Reading input data")
        return "Hello World!"
        
    def exec(self, prep_res):
        print(f"InputNode: Processing '{prep_res}'")
        return prep_res.upper()
        
    def post(self, shared, prep_res, exec_res):
        print(f"InputNode: Saving result '{exec_res}'")
        shared.set("uppercase", exec_res)
        return "default"

class ProcessNode(Node):
    def prep(self, shared):
        print("ProcessNode: Getting uppercase text")
        return shared.get("uppercase")
        
    def exec(self, prep_res):
        print(f"ProcessNode: Adding exclamation to '{prep_res}'")
        return f"{prep_res}!!!"
        
    def post(self, shared, prep_res, exec_res):
        print(f"ProcessNode: Saving result '{exec_res}'")
        shared.set("result", exec_res)
        return "default"

class OutputNode(Node):
    def prep(self, shared):
        print("OutputNode: Getting result")
        return shared.get("result")
        
    def exec(self, prep_res):
        print(f"OutputNode: Final result: '{prep_res}'")
        return prep_res
        
    def post(self, shared, prep_res, exec_res):
        print("OutputNode: Done")
        return "default"

def main():
    print("Creating flow with InputNode >> ProcessNode >> OutputNode")
    
    # Create a flow using the same syntax as the original Python version
    flow = Flow(
        InputNode() >> ProcessNode() >> OutputNode()
    )
    
    # Create a shared store
    shared = SharedStore()
    
    # Run the flow
    print("\nRunning flow...")
    flow.run(shared)
    
    # Check the result
    print("\nFinal result in store:", shared.get("result"))

if __name__ == "__main__":
    main() 