use crate::tile_provider::{GraphTileProvider, GraphTileProviderError};
use crate::{graph_tile::GraphTile, GraphId};
use std::io::ErrorKind;
use std::path::PathBuf;
use zerocopy::IntoBytes;

pub struct DirectoryTileProvider {
    base_directory: PathBuf,
}

impl DirectoryTileProvider {
    pub fn new(base_directory: PathBuf) -> Self {
        DirectoryTileProvider { base_directory }
    }
}

impl GraphTileProvider for DirectoryTileProvider {
    fn get_tile(&self, graph_id: &GraphId) -> Result<GraphTile, GraphTileProviderError> {
        // Build up the path from the base directory + tile ID components
        // TODO: Do we want to move the base ID check inside file_path?
        let base_graph_id = graph_id.tile_base_id();
        // TODO: Cache
        let path = self.base_directory.join(base_graph_id.file_path("gph")?);
        // Open the file and read all bytes into a buffer
        // TODO: Handle compressed tiles
        let data = std::fs::read(path).map_err(|e| match e.kind() {
            ErrorKind::NotFound => GraphTileProviderError::TileDoesNotExist,
            _ => GraphTileProviderError::IoError(e),
        })?;
        // Construct a graph tile with the bytes
        Ok(GraphTile::try_from(data.as_bytes())?)
    }
}

#[cfg(test)]
mod test {
    use super::DirectoryTileProvider;
    use crate::tile_provider::GraphTileProvider;
    use crate::GraphId;
    use std::path::PathBuf;

    #[test]
    fn test_get_tile() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("andorra-tiles");
        let provider = DirectoryTileProvider::new(base);
        let graph_id = GraphId::try_from_components(0, 3015, 0).expect("Unable to create graph ID");
        let tile = provider.get_tile(&graph_id).expect("Unable to get tile");

        // Minimally test that we got the correct tile
        assert_eq!(tile.header.graph_id(), graph_id);
        assert_eq!(tile.header.graph_id().value(), graph_id.value());
    }
}
