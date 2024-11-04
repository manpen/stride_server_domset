#!/usr/bin/env python3
import time
import sys

nodes = None
edges = None
for line in sys.stdin:
    line = line.strip()
    if line.startswith("p ds"):
        _, _, nodes, edges = line.replace("  ", " ").split()
        nodes = int(nodes)
        edges = int(edges)
        break

if nodes is None:
    print("Failed to parse header")
    sys.exit(1)

solution = [x for x in range(1, nodes+1) if x % 3 == 0]


# output solution
solution = [len(solution)] + solution
print("\n".join(str(x) for x in solution))

