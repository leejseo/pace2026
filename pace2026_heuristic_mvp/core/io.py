from typing import Optional, Set
import dataclasses
from .tree import OriginalNode

class ParseError(ValueError): pass

class NewickParser:
    def __init__(self, text: str) -> None:
        self.text = text
        self.i = 0

    def parse(self) -> OriginalNode:
        node = self._parse_subtree()
        if self.i >= len(self.text) or self.text[self.i] != ";":
            raise ParseError("expected ';' at end")
        self.i += 1
        return node

    def _parse_subtree(self) -> OriginalNode:
        ch = self.text[self.i]
        if ch.isdigit():
            start = self.i
            while self.i < len(self.text) and self.text[self.i].isdigit(): self.i += 1
            return OriginalNode(label=int(self.text[start : self.i]))
        self.i += 1 # '('
        left = self._parse_subtree()
        self.i += 1 # ','
        right = self._parse_subtree()
        self.i += 1 # ')'
        return OriginalNode(left=left, right=right)

@dataclasses.dataclass(frozen=True)
class Instance:
    n_leaves: int
    tree1: OriginalNode
    tree2: OriginalNode

def collect_original_labels(node: OriginalNode) -> set[int]:
    if node.is_leaf: return {int(node.label)}
    return collect_original_labels(node.left) | collect_original_labels(node.right)

def parse_instance(text: str) -> Instance:
    lines = [line.strip() for line in text.splitlines() if line.strip() and not line.startswith("#")]
    newicks = [line for line in lines if line.endswith(";")]
    tree1 = NewickParser(newicks[0]).parse()
    tree2 = NewickParser(newicks[1]).parse()
    labels = collect_original_labels(tree1)
    return Instance(n_leaves=len(labels), tree1=tree1, tree2=tree2)

def render_expansion(exp) -> str:
    if isinstance(exp, int): return str(exp)
    return f"({render_expansion(exp[0])},{render_expansion(exp[1])})"
