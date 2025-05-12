from langgraph import StateGraph, END
from ..nodes.plan_node     import plan_node
from ..nodes.code_node     import code_node
from ..nodes.validate_node import validate_node
from ..nodes.pr_node       import pr_node

Graph = StateGraph(dict, dict)
Graph.add_state("plan",     plan_node)
Graph.add_state("code",     code_node)
Graph.add_state("validate", validate_node)
Graph.add_state("pr",       pr_node)

Graph.add_edge("plan", "code")            # first diff
Graph.add_edge("code", "validate")         # compile / tests / lint
# Loop back to code when validate fails (green==False)
Graph.add_conditional_edges(
    "validate",
    lambda s: "code" if not s.get("green", False) else "pr",
)
Graph.add_edge("pr", END)
workflow = Graph.compile() 