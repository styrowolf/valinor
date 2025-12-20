use std::{hash::Hash, path::PathBuf};

use valhalla_graphtile::{
    Access, GraphId, Surface,
    graph_tile::{DirectedEdge, GraphTile, GraphTileView, NodeInfo},
    tile_provider::{GraphTileProvider, TarballTileProvider},
};

#[derive(Clone, Debug)]
pub struct EdgeLabel {
    edge_id: GraphId,
    cost: Cost,
}

impl Hash for EdgeLabel {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.edge_id.hash(state);
    }
}

impl PartialEq for EdgeLabel {
    fn eq(&self, other: &Self) -> bool {
        self.edge_id == other.edge_id
    }
}

impl Eq for EdgeLabel {}

#[derive(Clone, Copy, Debug)]
pub struct Cost {
    cost: f64,
    secs: f64,
}

impl std::ops::Add for Cost {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            cost: self.cost + other.cost,
            secs: self.secs + other.secs,
        }
    }
}

pub trait Costing {
    fn node_allowed(&self, node: &NodeInfo) -> bool;
    fn edge_allowed(&self, edge: &DirectedEdge, pred: &EdgeLabel) -> bool;
    fn edge_cost(&self, edge: &DirectedEdge) -> Cost;
    fn transition_cost(&self, edge: &DirectedEdge, pred: &EdgeLabel, node: &NodeInfo) -> Cost;
    fn allow_snap(&self, edge: &DirectedEdge) -> bool;
}

pub struct SimpleCosting {
    access: Access,
}

impl Costing for SimpleCosting {
    fn node_allowed(&self, node: &NodeInfo) -> bool {
        node.access().contains(self.access)
    }

    fn edge_allowed(&self, edge: &DirectedEdge, _pred: &EdgeLabel) -> bool {
        edge.forward_access().contains(self.access) && edge.surface() != Surface::Impassable
    }

    fn edge_cost(&self, edge: &DirectedEdge) -> Cost {
        Cost {
            cost: edge.length() as f64,
            secs: edge.length() as f64 / (edge.speed() as f64 / 3.6),
        }
    }

    fn transition_cost(&self, _edge: &DirectedEdge, _pred: &EdgeLabel, _node: &NodeInfo) -> Cost {
        Cost {
            cost: 0.0,
            secs: 0.0,
        }
    }

    fn allow_snap(&self, edge: &DirectedEdge) -> bool {
        edge.forward_access().contains(self.access) && edge.surface() != Surface::Impassable
    }
}

trait TileExt {
    fn get_outbound_edges_from_node_with_ids(
        &self,
        node_id: GraphId,
        node_info: &NodeInfo,
    ) -> Vec<(GraphId, DirectedEdge)>;
}

impl<'a> TileExt for GraphTileView<'a> {
    fn get_outbound_edges_from_node_with_ids(
        &self,
        node_id: GraphId,
        node_info: &NodeInfo,
    ) -> Vec<(GraphId, DirectedEdge)> {
        let start = node_info.edge_index() as usize;
        let count = usize::from(node_info.edge_count());

        let ids = (start..(start + count)).map(|idx| node_id.with_index(idx as u64).unwrap());
        let edges = self.get_outbound_edges_from_node(node_info);

        ids.zip(edges.iter().cloned()).collect()
    }
}

#[test]
fn pathfinding_test() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("valhalla-graphtile")
        .join("fixtures")
        .join("andorra-tiles.tar");
    let provider: TarballTileProvider<false> =
        TarballTileProvider::new(path).expect("Unable to init tile provider");
    let graph_id = GraphId::try_from_components(0, 3015, 0).expect("Unable to create graph ID");
    assert!(provider.get_pointer_for_tile_containing(graph_id).is_ok());

    let origin = GraphId::try_from_id(36003929656).expect("Invalid graph ID");
    let destination = GraphId::try_from_id(64491642424).expect("Invalid graph ID");

    let path_origin = EdgeLabel {
        edge_id: origin,
        cost: Cost {
            cost: 0.0,
            secs: 0.0,
        },
    };

    let successors = |label: &EdgeLabel| {
        let costing = SimpleCosting {
            access: Access::Auto,
        };
        provider.with_tile_containing_or_panic(label.edge_id, |tile| {
            let edge = tile.get_directed_edge(label.edge_id).unwrap();
            let node_id = edge.end_node_id();
            let node = provider.with_tile_containing_or_panic(node_id, |tile| {
                tile.get_node(node_id).unwrap().clone()
            });
            let edges = provider.with_tile_containing_or_panic(node_id, |tile| {
                tile.get_outbound_edges_from_node_with_ids(node_id, &node)
            });

            let out: Vec<_> = edges
                .iter()
                .filter_map(|(edge_id, edge)| {
                    let end_node_id = edge.end_node_id();
                    let end_node = provider.with_tile_containing_or_panic(end_node_id, |tile| {
                        tile.get_node(end_node_id).unwrap().clone()
                    });
                    if costing.edge_allowed(edge, label) && costing.node_allowed(&end_node) {
                        let cost = costing.edge_cost(edge)
                            + costing.transition_cost(edge, label, &node)
                            + label.cost;
                        Some((
                            EdgeLabel {
                                edge_id: *edge_id,
                                cost: cost.clone(),
                            },
                            (cost.cost * 10.0_f64.powi(6)) as u64,
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            out.into_iter()
        })
    };

    let result = pathfinding::prelude::astar(
        &path_origin,
        successors,
        |label| 0,
        |label| label.edge_id == destination,
    );

    for (i, edge_label) in result.unwrap().0.iter().enumerate() {
        eprintln!("{:?}", edge_label);
        provider.with_tile_containing_or_panic(edge_label.edge_id, |tile| {
            let edge = tile.get_directed_edge(edge_label.edge_id).unwrap();
            let edge_info = tile.get_edge_info(edge).unwrap();
            let mut raw_shape = edge_info.decode_raw_shape().unwrap();
            if !edge.edge_info_is_forward() {
                raw_shape.reverse();
            }
            println!(
                "{}",
                serde_json::json!({
                    "edge_id": edge_label.edge_id.value(),
                    "idx": i,
                    "shape": raw_shape.into_iter().map(|c| [c.x, c.y]).collect::<Vec<_>>(),
                })
                .to_string()
            );
        })
    }
}
