use valhalla_graphtile::graph_tile::{DirectedEdge, NodeInfo};

pub struct EdgeLabel {}

pub struct Cost {
    cost: f64,
    secs: f64,
}

pub trait Costing {
    fn node_allowed(&self, node: &NodeInfo) -> bool;
    fn edge_allowed(&self, edge: &DirectedEdge, pred: &EdgeLabel) -> bool;
    fn edge_cost(&self, edge: &DirectedEdge) -> Cost;
    fn transition_cost(&self, edge: &DirectedEdge, pred: &EdgeLabel) -> Cost;
    fn allow_snap(&self, edge: &DirectedEdge) -> bool;
}

pub struct SimpleCosting {
    access: Access,
}
