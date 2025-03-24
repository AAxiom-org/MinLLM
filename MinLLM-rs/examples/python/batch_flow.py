#!/usr/bin/env python3
"""
Batch processing example demonstrating the MinLLM package.
This replicates the exact same API as the original Python version.
"""

from minllm import Node, BatchNode, BatchFlow, SharedStore

class InputBatchNode(BatchNode):
    def prep(self, shared):
        print("InputBatchNode: Creating batch of parameters")
        # Return a list of parameter dicts to be processed
        return [
            {'item': 'apple', 'color': 'red'},
            {'item': 'banana', 'color': 'yellow'},
            {'item': 'grape', 'color': 'purple'}
        ]
    
class ProcessItemNode(Node):
    def prep(self, shared):
        print(f"ProcessItemNode: Processing {self.params}")
        return self.params
        
    def exec(self, params):
        print(f"ProcessItemNode: Formatting {params['item']}")
        return f"The {params['item']} is {params['color']}"
        
    def post(self, shared, prep_res, exec_res):
        print(f"ProcessItemNode: Saving result '{exec_res}'")
        # Append to results list in shared store
        results = shared.get('results') or []
        results.append(exec_res)
        shared.set('results', results)
        return "default"

class OutputResultsNode(Node):
    def prep(self, shared):
        print("OutputResultsNode: Getting all results")
        return shared.get('results')
        
    def exec(self, results):
        print(f"OutputResultsNode: Combining {len(results)} results")
        return "\n".join(results)
        
    def post(self, shared, prep_res, exec_res):
        print(f"OutputResultsNode: Saving final output")
        shared.set('final_output', exec_res)
        return "default"

def main():
    print("Creating batch flow")
    
    # Create a batch flow
    flow = BatchFlow(
        InputBatchNode() >> ProcessItemNode() >> OutputResultsNode()
    )
    
    # Create a shared store
    shared = SharedStore()
    
    # Run the flow
    print("\nRunning batch flow...")
    flow.run(shared)
    
    # Check the result
    print("\nFinal output:")
    print(shared.get("final_output"))

if __name__ == "__main__":
    main() 