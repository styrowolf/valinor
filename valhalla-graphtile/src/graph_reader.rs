use crate::tile_provider::{GraphTileProvider, GraphTileProviderError};
use crate::{graph_tile::GraphTile, GraphId};

pub struct GraphReader<T: GraphTileProvider> {
    tile_provider: T,
    // TODO: Cache
}

impl<T: GraphTileProvider> GraphTileProvider for GraphReader<T> {
    fn get_tile_containing(&self, graph_id: &GraphId) -> Result<GraphTile, GraphTileProviderError> {
        self.tile_provider.get_tile_containing(graph_id)
    }
}
