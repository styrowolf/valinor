use bitfield_struct::bitfield;
use zerocopy_derive::FromBytes;

#[derive(FromBytes)]
#[bitfield(u32)]
struct EdgeIndex {
    #[bits(22)]
    edge_index: u32,
    #[bits(10)]
    _spare: u16,
}

#[derive(FromBytes, Debug)]
pub struct TurnLane {
    edge_index: EdgeIndex,
    /// The offset into the [`GraphTile`](super::GraphTile) text list
    /// which contains the text for this turn lane.
    pub text_offset: u32,
}

/// Holds turn lane information at the end of a directed edge.
///
/// Turn lane text is stored in the [`GraphTile`](super::GraphTile) text list
/// and the offset is stored within the TurnLane structure.
impl TurnLane {
    /// Gets the index (within the same tile) of the directed edge that this sign applies to.
    #[inline]
    pub fn directed_edge_index(&self) -> u32 {
        self.edge_index.edge_index()
    }
}

#[cfg(test)]
mod tests {
    use crate::graph_tile::TEST_GRAPH_TILE;

    #[test]
    fn test_parse_turn_lane_count() {
        let tile = &*TEST_GRAPH_TILE;

        assert_eq!(
            tile.turn_lanes.len(),
            tile.header.turn_lane_count() as usize
        );
    }

    #[test]
    fn test_parse_turn_lanes() {
        let tile = &*TEST_GRAPH_TILE;

        insta::assert_debug_snapshot!(tile.turn_lanes);
    }
}
