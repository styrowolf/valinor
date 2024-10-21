use thiserror::Error;
use zerocopy::{transmute, try_transmute};

use bytes::Bytes;
use enumset::EnumSet;
#[cfg(test)]
use std::sync::LazyLock;

// To keep files manageable, we will keep internal modules specific to each mega-struct,
// and publicly re-export the public type.
// Each struct should be tested on fixture tiles to a reasonable level of confidence.
// Leverage snapshot tests where possible, compare with Valhalla C++ results,
// and test the first and last elements of variable length vectors
// with snapshot tests.

mod access_restriction;
mod admin;
mod directed_edge;
mod edge_info;
mod header;
mod node;
mod sign;
mod transit;
mod turn_lane;

use crate::{
    graph_id::{GraphId, InvalidGraphIdError},
    transmute_variable_length_data, try_transmute_variable_length_data, Access,
};
pub use access_restriction::{AccessRestriction, AccessRestrictionType};
pub use admin::Admin;
pub use directed_edge::{DirectedEdge, DirectedEdgeExt};
pub use edge_info::EdgeInfo;
pub use header::GraphTileHeader;
pub use node::{NodeInfo, NodeTransition};
pub use sign::{Sign, SignType};
pub use transit::{TransitDeparture, TransitRoute, TransitSchedule, TransitStop, TransitTransfer};
pub use turn_lane::TurnLane;

#[derive(Debug, Error)]
pub enum GraphTileError {
    #[error("Unable to extract a slice of the correct length; the tile data is malformed.")]
    SliceArrayConversion(#[from] std::array::TryFromSliceError),
    #[error("The byte sequence is not valid for this type.")]
    ValidityError,
    #[error("Invalid graph ID.")]
    GraphIdParseError(#[from] InvalidGraphIdError),
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
    memory: Bytes,
    /// Header with various metadata about the tile and internal sizes.
    pub header: GraphTileHeader,
    /// The list of nodes in the graph tile.
    nodes: Vec<NodeInfo>,
    /// The list of transitions between nodes on different levels.
    transitions: Vec<NodeTransition>,
    directed_edges: Vec<DirectedEdge>,
    ext_directed_edges: Vec<DirectedEdgeExt>,
    access_restrictions: Vec<AccessRestriction>,
    transit_departures: Vec<TransitDeparture>,
    transit_stops: Vec<TransitStop>,
    transit_routes: Vec<TransitRoute>,
    transit_schedules: Vec<TransitSchedule>,
    transit_transfers: Vec<TransitTransfer>,
    signs: Vec<Sign>,
    turn_lanes: Vec<TurnLane>,
    admins: Vec<Admin>,
    edge_bins: GraphId,
    // TODO: Complex forward restrictions
    // TODO: Complex reverse restrictions
    // TODO: Street names (names here)
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

    /// Gets edge info for a directed edge.
    pub fn get_edge_info(&self, directed_edge: &DirectedEdge) -> Result<EdgeInfo, GraphTileError> {
        let edge_info_start = self.header.edge_info_offset as usize;
        let edge_info_offset = directed_edge.edge_info_offset() as usize;

        // TODO: Text slice as a separate property or a function?
        let text_start = self.header.text_list_offset as usize;
        let text_size = self.header.lane_connectivity_offset as usize - text_start;

        Ok(EdgeInfo::try_from((
            self.memory.slice(edge_info_start + edge_info_offset..),
            self.memory.slice(text_start..text_start + text_size),
        ))?)
    }
}

// TODO: Feels like this could be a macro
impl TryFrom<Bytes> for GraphTile {
    type Error = GraphTileError;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        let value = bytes.as_ref();
        // Get the byte range of the header so we can transmute it
        const HEADER_SIZE: usize = size_of::<GraphTileHeader>();
        let header_range = 0..HEADER_SIZE;

        // Save the pointer to the list of nodes (the range is consumed)
        let offset = header_range.end;

        // Header
        let header_slice: [u8; HEADER_SIZE] = value[header_range].try_into()?;
        let header: GraphTileHeader = transmute!(header_slice);

        // All the variably sized data arrays

        let (nodes, offset) =
            transmute_variable_length_data!(NodeInfo, value, offset, header.node_count() as usize)?;

        let (transitions, offset) = transmute_variable_length_data!(
            NodeTransition,
            value,
            offset,
            header.transition_count() as usize
        )?;

        let (directed_edges, offset) = try_transmute_variable_length_data!(
            DirectedEdge,
            value,
            offset,
            header.directed_edge_count() as usize
        )?;

        let directed_edge_ext_count = if header.has_ext_directed_edge() {
            header.directed_edge_count() as usize
        } else {
            0
        };
        let (ext_directed_edges, offset) = transmute_variable_length_data!(
            DirectedEdgeExt,
            value,
            offset,
            directed_edge_ext_count
        )?;

        let (access_restrictions, offset) = try_transmute_variable_length_data!(
            AccessRestriction,
            value,
            offset,
            header.access_restriction_count() as usize
        )?;

        let (transit_departures, offset) = transmute_variable_length_data!(
            TransitDeparture,
            value,
            offset,
            header.departure_count() as usize
        )?;

        let (transit_stops, offset) = transmute_variable_length_data!(
            TransitStop,
            value,
            offset,
            header.stop_count() as usize
        )?;

        let (transit_routes, offset) = transmute_variable_length_data!(
            TransitRoute,
            value,
            offset,
            header.route_count() as usize
        )?;

        let (transit_schedules, offset) = transmute_variable_length_data!(
            TransitSchedule,
            value,
            offset,
            header.schedule_count() as usize
        )?;

        let (transit_transfers, offset) = transmute_variable_length_data!(
            TransitTransfer,
            value,
            offset,
            header.transfer_count() as usize
        )?;

        let (signs, offset) =
            try_transmute_variable_length_data!(Sign, value, offset, header.sign_count() as usize)?;

        let (turn_lanes, offset) = try_transmute_variable_length_data!(
            TurnLane,
            value,
            offset,
            header.turn_lane_count() as usize
        )?;

        let (admins, offset) = try_transmute_variable_length_data!(
            Admin,
            value,
            offset,
            header.admin_count() as usize
        )?;

        const U64_SIZE: usize = size_of::<u64>();
        let slice: [u8; U64_SIZE] = (&value[offset..offset + U64_SIZE]).try_into()?;
        let raw_graph_id = transmute!(slice);
        let edge_bins = GraphId::try_from_id(raw_graph_id)?;

        Ok(Self {
            memory: bytes,
            header,
            nodes,
            transitions,
            directed_edges,
            ext_directed_edges,
            access_restrictions,
            transit_departures,
            transit_stops,
            transit_routes,
            transit_schedules,
            transit_transfers,
            signs,
            turn_lanes,
            admins,
            edge_bins,
        })
    }
}

#[cfg(test)]
const TEST_GRAPH_TILE_ID: GraphId = unsafe { GraphId::from_components_unchecked(0, 3015, 0) };

#[cfg(test)]
static TEST_GRAPH_TILE: LazyLock<GraphTile> = LazyLock::new(|| {
    let relative_path = TEST_GRAPH_TILE_ID
        .file_path("gph")
        .expect("Unable to get relative path");
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join("andorra-tiles")
        .join(relative_path);
    let bytes = Bytes::from(std::fs::read(path).expect("Unable to read file"));

    GraphTile::try_from(bytes).expect("Unable to get tile")
});

#[cfg(test)]
mod tests {
    use crate::graph_tile::{TEST_GRAPH_TILE, TEST_GRAPH_TILE_ID};
    use crate::Access;
    use enumset::{enum_set, EnumSet};

    #[test]
    fn test_get_opp_edge_index() {
        let tile = &*TEST_GRAPH_TILE;

        let edges: Vec<_> = (0..u64::from(tile.header.directed_edge_count()))
            .map(|index| {
                let edge_id = TEST_GRAPH_TILE_ID
                    .with_index(index)
                    .expect("Invalid graph ID.");
                let opp_edge_index = tile
                    .get_opp_edge_index(&edge_id)
                    .expect("Unable to get opp edge index.");
                opp_edge_index
            })
            .collect();
        insta::assert_debug_snapshot!(edges);
    }

    #[test]
    fn test_get_access_restrictions() {
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

    #[test]
    fn test_edge_bins() {
        // TODO: TBH I don't actually understand how this is a valid graph tile. Clearly some black bit magic.
        let tile = &*TEST_GRAPH_TILE;
        assert_eq!(tile.edge_bins.level(), 0);
        assert_eq!(tile.edge_bins.tile_id(), 0);
        assert_eq!(tile.edge_bins.index(), 32000);
    }

    #[test]
    fn test_edge_info() {
        let tile = &*TEST_GRAPH_TILE;
        let first_edge_info = tile
            .get_edge_info(&tile.directed_edges[0])
            .expect("Unable to get edge info.");
        insta::assert_debug_snapshot!("first_edge_info", first_edge_info);
        insta::assert_debug_snapshot!("first_edge_info_decoded_shape", first_edge_info.shape());
        insta::assert_debug_snapshot!("first_edge_info_names", first_edge_info.get_names());

        let last_edge_info = tile
            .get_edge_info(&tile.directed_edges.last().unwrap())
            .expect("Unable to get edge info.");
        insta::assert_debug_snapshot!("last_edge_info", last_edge_info);

        // Edge chosen somewhat randomly; it happens to have multiple names.
        let other_edge_info = tile
            .get_edge_info(&tile.directed_edges[2000])
            .expect("Unable to get edge info.");
        insta::assert_debug_snapshot!("other_edge_info", other_edge_info);
        insta::assert_debug_snapshot!("other_edge_info_names", other_edge_info.get_names());
    }
}
