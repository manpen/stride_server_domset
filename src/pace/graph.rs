pub type Node = u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Edge(pub Node, pub Node);
pub type NumNodes = Node;
pub type NumEdges = u64;
