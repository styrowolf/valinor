use crate::tile_provider::{GraphTileProvider, GraphTileProviderError};
use crate::{graph_tile::GraphTile, GraphId};
use bytes::Bytes;
use std::io::ErrorKind;
use std::path::PathBuf;

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
        let data = Bytes::from(std::fs::read(path).map_err(|e| match e.kind() {
            ErrorKind::NotFound => GraphTileProviderError::TileDoesNotExist,
            _ => GraphTileProviderError::IoError(e),
        })?);
        // Construct a graph tile with the bytes
        Ok(GraphTile::new(data)?)
    }
}

#[cfg(test)]
mod test {
    use super::DirectoryTileProvider;
    use crate::tile_provider::GraphTileProvider;
    use crate::{GraphId, BIN_COUNT};
    use geo::coord;
    use std::path::PathBuf;

    #[test]
    fn test_parse_header() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("andorra-tiles");
        let provider = DirectoryTileProvider::new(base);
        let graph_id = GraphId::try_from_components(0, 3015, 0).expect("Unable to create graph ID");
        let tile = provider.get_tile(&graph_id).expect("Unable to get tile");
        let header = tile.header;

        assert_eq!(header.graph_id(), graph_id.value());
        assert_eq!(header.density(), 0);
        assert_eq!(header.name_quality(), 0);
        assert_eq!(header.speed_quality(), 0);
        assert_eq!(header.exit_quality(), 0);
        assert_eq!(header.has_elevation(), false);
        assert_eq!(header.has_ext_directed_edge(), false);
        assert_eq!(header.sw_corner(), coord! {x: 0.0, y: 42.0 });
        assert_eq!(header.version(), "3.5.0");
        assert_eq!(header.dataset_id, 12218323114);
        assert_eq!(header.node_count(), 1343);
        assert_eq!(header.directed_edge_count(), 3426);
        assert_eq!(header.predicted_speeds_count(), 0);
        assert_eq!(header.transition_count(), 712);
        assert_eq!(header.turn_lane_count(), 6);
        // TODO: Build and test some transit tiles
        assert_eq!(header.transfer_count(), 0);
        assert_eq!(header.departure_count(), 0);
        assert_eq!(header.stop_count(), 0);
        assert_eq!(header.route_count(), 0);
        assert_eq!(header.schedule_count(), 0);
        assert_eq!(header.sign_count(), 0);
        assert_eq!(header.access_restriction_count(), 8);
        assert_eq!(header.admin_count(), 4);
        assert_eq!(header.reserved, 0);
        assert_eq!(header.complex_restriction_forward_offset, 213632);
        assert_eq!(header.complex_restriction_reverse_offset, 213632);
        assert_eq!(header.edge_info_offset, 213632);
        assert_eq!(header.text_list_offset, 304836);
        assert_eq!(header.create_date, 0);
        assert_eq!(header.bin_offsets, [0; BIN_COUNT as usize]);
        assert_eq!(header.lane_connectivity_offset, 305888);
        assert_eq!(header.predicted_speeds_offset, 0);
        assert_eq!(header.tile_size, 305888);
    }
}
