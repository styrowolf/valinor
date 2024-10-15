use thiserror::Error;
use zerocopy::{transmute, try_transmute};

use enumset::EnumSet;
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

mod access_restriction;
mod directed_edge;
mod header;
mod node;
mod transit;

use crate::{Access, GraphId};
pub use access_restriction::{AccessRestriction, AccessRestrictionType};
pub use directed_edge::{DirectedEdge, DirectedEdgeExt};
pub use header::GraphTileHeader;
pub use node::{NodeInfo, NodeTransition};
pub use transit::{TransitDeparture, TransitStop, TransitRoute, TransitSchedule, TransitTransfer};

/// Transmutes variable length data into a Vec<T>.
/// This can't be written as a function because the const generics
/// require explicit types and that context isn't available from function generic params.
macro_rules! transmute_variable_length_data {
    ($type:ty, $data:expr, $item_count:expr) => {{
        const PTR_SIZE: usize = size_of::<$type>();
        (0..$item_count)
            .map(|i| {
                let range = PTR_SIZE * i..PTR_SIZE * (i + 1);
                let slice: [u8; PTR_SIZE] = $data[range].try_into()?;
                Ok(transmute!(slice))
            })
            .collect::<Result<_, GraphTileError>>()
    }};
}

/// Tries to transmute variable length data into a Vec<T>.
/// Analogous to [`transmute_variable_length_data`],
/// but for types implementing [`zerocopy::TryFromBytes`]
/// rather than [`zerocopy::FromBytes`].
macro_rules! try_transmute_variable_length_data {
    ($type:ty, $data:expr, $item_count:expr) => {{
        const PTR_SIZE: usize = size_of::<$type>();
        (0..$item_count)
            .map(|i| {
                let range = PTR_SIZE * i..PTR_SIZE * (i + 1);
                let slice: [u8; PTR_SIZE] = $data[range].try_into()?;
                try_transmute!(slice).map_err(|_| GraphTileError::ValidityError)
            })
            .collect::<Result<_, GraphTileError>>()
    }};
}

#[derive(Debug, Error)]
pub enum GraphTileError {
    #[error("Unable to extract a slice of the correct length; the tile data is malformed.")]
    SliceArrayConversion(#[from] std::array::TryFromSliceError),
    #[error("The byte sequence is not valid for this type.")]
    ValidityError,
}

#[derive(Debug, Error)]
pub enum LookupError {
    #[error("Mismatched base; the graph ID cannot exist in this tile.")]
    MismatchedBase,
    #[error("The feature at the index specified does not exist in this tile.")]
    InvalidIndex,
}

/// A tile within the Valhalla hierarchical tile graph.
pub struct GraphTile {
    /// Header with various metadata about the tile and internal sizes.
    pub header: GraphTileHeader,
    /// The list of nodes in the graph tile.
    nodes: Vec<NodeInfo>,
    /// The list of transitions between nodes on different levels.
    transitions: Vec<NodeTransition>,
    directed_edges: Vec<DirectedEdge>,
    ext_directed_edges: Vec<DirectedEdgeExt>,
    access_restrictions: Vec<AccessRestriction>,
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
    // TODO: Street names (names here)
    // TODO: Edge bins
    // TODO: Lane connectivity
    // TODO: Predicted speeds
    // TODO: Stop one stops(?)
    // TODO: Route one stops(?)
    // TODO: Operator one stops(?)
}

impl GraphTile {
    /// Gets the Graph ID of the tile.
    #[inline]
    pub fn graph_id(&self) -> GraphId {
        self.header.graph_id()
    }

    /// Does the supplied graph ID belong in this tile?
    ///
    /// A true result does not necessarily guarantee that an object with this ID exists,
    /// but in that case, either you've cooked up an invalid ID for fun,
    /// or the graph is invalid.
    pub fn may_contain_id(&self, id: &GraphId) -> bool {
        id.tile_base_id() == self.graph_id().tile_base_id()
    }

    /// Gets a reference to a node in this tile with the given graph ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the graph ID cannot be contained in this tile
    /// or the index is invalid.
    pub fn get_node(&self, id: &GraphId) -> Result<&NodeInfo, LookupError> {
        if self.may_contain_id(id) {
            self.nodes
                .get(id.index() as usize)
                .ok_or(LookupError::InvalidIndex)
        } else {
            Err(LookupError::MismatchedBase)
        }
    }

    /// Gets a reference to the directed edge in this tile by graph ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the graph ID cannot be contained in this tile
    /// or the index is invalid.
    pub fn get_directed_edge(&self, id: &GraphId) -> Result<&DirectedEdge, LookupError> {
        if self.may_contain_id(id) {
            self.directed_edges
                .get(id.index() as usize)
                .ok_or(LookupError::InvalidIndex)
        } else {
            Err(LookupError::MismatchedBase)
        }
    }

    /// Gets the opposing edge index, if and only if it exists in this tile.
    pub fn get_opp_edge_index(&self, graph_id: &GraphId) -> Result<u32, LookupError> {
        let edge = self.get_directed_edge(graph_id)?;

        // The edge might leave the tile, so we have to do a complicated lookup
        let end_node_id = edge.end_node_id();
        let opp_edge_index = edge.opposing_edge_index();

        // TODO: Probably a cleaner pattern here?
        let node_edge_index = self.get_node(&end_node_id).map(|n| n.edge_index())?;

        Ok(node_edge_index + opp_edge_index)
    }

    /// Gets a reference to an extended directed edge in this tile by graph ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the graph ID cannot be contained in this tile
    /// or the index is invalid.
    pub fn get_ext_directed_edge(&self, id: &GraphId) -> Result<&DirectedEdgeExt, LookupError> {
        if self.may_contain_id(id) {
            self.ext_directed_edges
                .get(id.index() as usize)
                .ok_or(LookupError::InvalidIndex)
        } else {
            Err(LookupError::MismatchedBase)
        }
    }

    /// Gets access restriction for a directed edge
    /// which apply to *any* of the supplied access modes (ex: auto, bicycle, etc.).
    pub fn get_access_restrictions(
        &self,
        directed_edge_index: u32,
        access_modes: EnumSet<Access>,
    ) -> Vec<&AccessRestriction> {
        // Find the point such that all elements below index are less than directed_edge_index
        // (NOTE: the list is pre-sorted during tile building)
        let index = self
            .access_restrictions
            .partition_point(|e| e.edge_index() < directed_edge_index);

        self.access_restrictions
            .iter()
            // Iterate over the relevant objects (this is an empty iterator if index >= length)
            .skip(index)
            .take_while(|e| e.edge_index() == directed_edge_index)
            .filter(|e| !e.affected_access_modes().is_disjoint(access_modes))
            .collect()
    }
}

impl TryFrom<&[u8]> for GraphTile {
    type Error = GraphTileError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        // Get the byte range of the header so we can transmute it
        let header_range = 0..size_of::<GraphTileHeader>();

        // Save the pointer to the list of nodes (the range is consumed)
        let nodes_offset = header_range.end;

        // Header
        let header_slice: [u8; size_of::<GraphTileHeader>()] = value[header_range].try_into()?;
        let header: GraphTileHeader = transmute!(header_slice);

        // TODO: Clean this repetitive mess up with a macro or something?

        // Nodes
        let node_count = header.node_count() as usize;
        let nodes = transmute_variable_length_data!(NodeInfo, &value[nodes_offset..], node_count)?;

        // Transitions
        let transitions_offset = nodes_offset + (size_of::<NodeInfo>() * node_count);
        let transition_count = header.transition_count() as usize;
        let transitions = transmute_variable_length_data!(
            NodeTransition,
            &value[transitions_offset..],
            transition_count
        )?;

        // Directed edges
        let directed_edges_offset =
            transitions_offset + (size_of::<NodeTransition>() * transition_count);
        let directed_edge_count = header.directed_edge_count() as usize;
        let directed_edges = try_transmute_variable_length_data!(
            DirectedEdge,
            &value[directed_edges_offset..],
            directed_edge_count
        )?;

        // Extended directed edges
        let directed_edges_ext_offset =
            directed_edges_offset + (size_of::<DirectedEdge>() * directed_edge_count);
        let directed_edge_ext_count = if header.has_ext_directed_edge() {
            header.directed_edge_count() as usize
        } else {
            0
        };
        let ext_directed_edges = transmute_variable_length_data!(
            DirectedEdgeExt,
            &value[directed_edges_ext_offset..],
            directed_edge_ext_count
        )?;

        // Access restrictions
        let access_restrictions_offset =
            directed_edges_ext_offset + (size_of::<DirectedEdgeExt>() * directed_edge_ext_count);
        let access_restriction_count = header.access_restriction_count() as usize;
        let access_restrictions = try_transmute_variable_length_data!(
            AccessRestriction,
            &value[access_restrictions_offset..],
            access_restriction_count
        )?;

        Ok(Self {
            header,
            nodes,
            transitions,
            directed_edges,
            ext_directed_edges,
            access_restrictions,
        })
    }
}

#[cfg(test)]
static TEST_GRAPH_TILE_ID: LazyLock<GraphId> =
    LazyLock::new(|| GraphId::try_from_components(0, 3015, 0).expect("Unable to create graph ID"));

#[cfg(test)]
static TEST_GRAPH_TILE: LazyLock<GraphTile> = LazyLock::new(|| {
    let graph_id = &*TEST_GRAPH_TILE_ID;
    let relative_path = graph_id
        .file_path("gph")
        .expect("Unable to get relative path");
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("andorra-tiles")
        .join(relative_path);
    let bytes = std::fs::read(path).expect("Unable to read file");

    GraphTile::try_from(bytes.as_bytes()).expect("Unable to get tile")
});

#[cfg(test)]
mod tests {
    use crate::graph_tile::{TEST_GRAPH_TILE, TEST_GRAPH_TILE_ID};
    use crate::Access;
    use enumset::{enum_set, EnumSet};

    #[test]
    fn test_get_opp_edge_index() {
        let graph_id = &*TEST_GRAPH_TILE_ID;
        let tile = &*TEST_GRAPH_TILE;

        let edges: Vec<_> = (0..u64::from(tile.header.directed_edge_count()))
            .map(|index| {
                let edge_id = graph_id.with_index(index).expect("Invalid graph ID.");
                let opp_edge_index = tile
                    .get_opp_edge_index(&edge_id)
                    .expect("Unable to get opp edge index.");
                opp_edge_index
            })
            .collect();
        insta::assert_debug_snapshot!(edges);
    }

    #[test]
    fn test_get_get_access_restrictions() {
        let graph_id = &*TEST_GRAPH_TILE_ID;
        let tile = &*TEST_GRAPH_TILE;

        let mut found_restriction_count = 0;
        let mut found_partial_subset_count = 0;
        for edge_index in 0..tile.header.directed_edge_count() {
            // Sanity check edge index with no access mode filter
            let all_edge_restrictions = tile.get_access_restrictions(edge_index, EnumSet::all());
            for restriction in &all_edge_restrictions {
                found_restriction_count += 1;
                assert_eq!(restriction.edge_index(), edge_index);
            }

            // In the tile fixture, all restrictions apply to trucks
            assert_eq!(
                all_edge_restrictions,
                tile.get_access_restrictions(edge_index, enum_set!(Access::Truck))
            );

            // Empty set should yield no access restrictions
            assert!(tile
                .get_access_restrictions(edge_index, EnumSet::empty())
                .is_empty());

            // This set includes one mode that matches 4 elements in the fixture tile,
            // and another mode that matches none.
            for restriction in
                tile.get_access_restrictions(edge_index, enum_set!(Access::Auto | Access::GolfCart))
            {
                found_partial_subset_count += 1;
                assert_eq!(restriction.edge_index(), edge_index);
            }
        }

        // Hard-coded bits based on the test fixture
        assert_eq!(found_restriction_count, 8);
        assert_eq!(found_partial_subset_count, 4);
    }
}
