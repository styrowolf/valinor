use std::rc::Rc;
use valhalla_graphtile::graph_tile::{DirectedEdge, GraphTile};
use valhalla_graphtile::GraphId;

// TODO: Do we need this?
pub struct EdgePointer<'a> {
    pub graph_id: GraphId,
    pub tile: Rc<GraphTile<'a>>,
}

impl EdgePointer<'_> {
    pub(crate) fn edge(&self) -> &DirectedEdge {
        self.tile
            .get_directed_edge(&self.graph_id)
            .expect("That wasn't supposed to happen...")
    }
}
