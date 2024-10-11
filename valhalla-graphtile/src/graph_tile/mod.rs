use thiserror::Error;
use zerocopy::{transmute, FromBytes};

#[cfg(test)]
use std::sync::LazyLock;

#[cfg(test)]
use zerocopy::IntoBytes;

// To keep files manageable, we will keep internal modules specific to each mega-struct,
// and publicly re-export the public type.
// Each struct should be tested on fixture tiles to a reasonable level of confidence.
// Leverage snapshot tests where possible, compare with Valhalla C++ results,
// and test the first and last elements of variable length vectors
// with snapshot tests.

mod header;
mod node_info;
mod node_transition;

pub use header::GraphTileHeader;
pub use node_info::NodeInfo;
pub use node_transition::NodeTransition;

/// Transmutes variable length data into a Vec<T>.
/// This can't be written as a function because the const generics
/// require explicit types and that context isn't available from function generic params.
macro_rules! transmute_variable_length_data {
    ($type:ty, $data:expr, $item_count:expr) => {
        {
            const PTR_SIZE: usize = size_of::<$type>();
            (0..$item_count).map(|i| {
                let range = PTR_SIZE * i..PTR_SIZE * (i + 1);
                let slice: [u8; PTR_SIZE] = $data[range].try_into()?;
                Ok(transmute!(slice))
            }).collect::<Result<_, GraphTileError>>()
        }
    };
}

#[derive(Debug, Error)]
pub enum GraphTileError {
    #[error("Unable to extract a slice of the correct length; the tile data is malformed.")]
    SliceArrayConversion(#[from] std::array::TryFromSliceError),
}

/// A tile within the Valhalla hierarchical tile graph.
pub struct GraphTile {
    /// Header with various metadata about the tile and internal sizes.
    pub header: GraphTileHeader,
    /// The list of nodes in the graph tile.
    pub nodes: Vec<NodeInfo>,
    /// The list of transitions between nodes on different levels.
    pub transitions: Vec<NodeTransition>,
    // TODO: Directed edges
    // TODO: Extended directed edges
    // TODO: Access restrictions (conditional?)
    // TODO: Transit departures
    // TODO: Transit stops
    // TODO: Transit routes
    // TODO: Transit schedules
    // TODO: Transit transfers
    // TODO: Signs
    // TODO: Turn lanes
    // TODO: Admins
    // TODO: Complex forward restrictions
    // TODO: Complex reverse restrictions
    // TODO: Edge info (shapes are here)
    // TODO: Street names
    // TODO: Edge bins
    // TODO: Lane connectivity
    // TODO: Predicted speeds
    // TODO: Stop one stops(?)
    // TODO: Route one stops(?)
    // TODO: Operator one stops(?)
}

impl TryFrom<&[u8]> for GraphTile {
    type Error = GraphTileError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        // Get the byte range of the header so we can transmute it
        let header_range = 0..size_of::<GraphTileHeader>();

        // Save the pointer to the list of nodes (the range is consumed)
        let node_info_start = header_range.end;

        // Parse the header
        let header_slice: [u8; size_of::<GraphTileHeader>()] = value[header_range].try_into()?;
        let header: GraphTileHeader = transmute!(header_slice);

        // Parse the list of nodes
        let node_count = header.node_count() as usize;
        let nodes = transmute_variable_length_data!(NodeInfo, &value[node_info_start..], node_count)?;

        // Parse the list of transitions
        let transition_info_start = node_info_start + (size_of::<NodeInfo>() * node_count);
        let transition_count = header.transition_count() as usize;
        let transitions = transmute_variable_length_data!(NodeTransition, &value[transition_info_start..], transition_count)?;
        // let transition_count = header.transition_count() as usize;
        // const TRANSITION_PTR_SIZE: usize = size_of::<NodeTransition>();
        // let nodes = (0..node_count).map(|i| {
        //     let range = (node_info_start + (NODE_PTR_SIZE * i))..node_info_start + (NODE_PTR_SIZE * (i + 1));
        //     let slice: [u8; size_of::<NodeInfo>()] = value[range].try_into()?;
        //     Ok(transmute!(slice))
        // }).collect::<Result<_, GraphTileError>>()?;

        // TODO: Parse the list of directed edges

        Ok(Self {
            header,
            nodes,
            transitions,
        })
    }
}

#[cfg(test)]
static TEST_GRAPH_TILE_ID: LazyLock<crate::GraphId> = LazyLock::new(|| {
    crate::GraphId::try_from_components(0, 3015, 0).expect("Unable to create graph ID")
});

#[cfg(test)]
static TEST_GRAPH_TILE: LazyLock<GraphTile> = LazyLock::new(|| {
    let graph_id = &*TEST_GRAPH_TILE_ID;
    let relative_path = graph_id.file_path("gph").expect("Unable to get relative path");
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("andorra-tiles")
        .join(relative_path);
    let bytes = std::fs::read(path).expect("Unable to read file");

    GraphTile::try_from(bytes.as_bytes()).expect("Unable to get tile")
});