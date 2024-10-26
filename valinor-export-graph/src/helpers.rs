use crate::models::EdgePointer;
use crate::Cli;
use bit_set::BitSet;
use std::borrow::Cow;
use std::collections::HashMap;
use std::rc::Rc;
use valhalla_graphtile::tile_provider::{GraphTileProvider, GraphTileProviderError};
use valhalla_graphtile::GraphId;

// TODO: Signature
pub(crate) fn next<'a, T: GraphTileProvider>(
    edge_pointer: EdgePointer<'a>,
    tile_set: HashMap<GraphId, usize>,
    processed_edges: &'a BitSet,
    tile_provider: T,
    cli: &'a Cli,
    names: &'a Vec<Cow<'a, str>>,
) -> Result<EdgePointer<'a>, GraphTileProviderError> {
    let end_node_id = edge_pointer.edge().end_node_id();
    // Get the tile containing the node
    let tile = if edge_pointer.tile.may_contain_id(&end_node_id) {
        edge_pointer.tile
    } else {
        Rc::new(tile_provider.get_tile_containing(&end_node_id)?)
    };

    // TODO (from Valhalla): in the case of multiple candidates, favor the ones with angles that are most straight

    let node = tile.get_node(&end_node_id)?;
    for i in 0..node.edge_count() as u32 {
        let id = tile.graph_id().with_index((node.edge_index() + i) as u64)?;

        // Check that the edge hasn't been processed yet
        let tileset_edge_running_total = *tile_set
            .get(&tile.graph_id())
            .expect("Unexpected tile (not in set).");
        if processed_edges.contains(tileset_edge_running_total + id.index() as usize) {
            continue;
        }

        let candidate = tile.get_directed_edge(&id)?;
        let candidate_edge_info = tile.get_edge_info(candidate)?;
        let candidate_names = candidate_edge_info.get_names();
        if cli.should_skip_edge(candidate, &candidate_names) && &candidate_names != names {
            continue;
        }

        // TODO: Optionally skip ferries (this feels like logic that's incorrectly copied; should be a function. Should exclude shortcuts too!
        // TODO: Check that names match
    }

    unimplemented!("Working on it...")
}
