use bitfield_struct::bitfield;
use zerocopy::{LE, U32};
use zerocopy_derive::{FromBytes, Immutable, Unaligned};

#[bitfield(u32,
    repr = U32<LE>,
    from = crate::conv_u32le::from_inner,
    into = crate::conv_u32le::into_inner
)]
#[derive(FromBytes, Immutable, Unaligned)]
struct EdgeIndex {
    #[bits(22, from = crate::conv_u32le::from_inner, into = crate::conv_u32le::into_inner)]
    edge_index: U32<LE>,
    #[bits(10)]
    _spare: U16<LE>,
}

#[derive(FromBytes, Immutable, Unaligned, Debug)]
#[repr(C)]
pub struct TurnLane {
    edge_index: EdgeIndex,
    /// The offset into the [`GraphTile`](super::GraphTile) text list
    /// which contains the text for this turn lane.
    pub text_offset: U32<LE>,
}

/// Holds turn lane information at the end of a directed edge.
///
/// Turn lane text is stored in the [`GraphTile`](super::GraphTile) text list
/// and the offset is stored within the [`TurnLane`] structure.
impl TurnLane {
    /// Gets the index (within the same tile) of the directed edge that this sign applies to.
    #[inline]
    pub const fn directed_edge_index(&self) -> u32 {
        self.edge_index.edge_index().get()
    }
}

#[cfg(test)]
mod tests {
    use crate::graph_tile::TEST_GRAPH_TILE;

    #[test]
    fn test_parse_turn_lane_count() {
        let owned_tile = &*TEST_GRAPH_TILE;
        let tile = owned_tile.as_tile();

        assert_eq!(
            tile.turn_lanes.len(),
            tile.header.turn_lane_count() as usize
        );
    }

    #[test]
    fn test_parse_turn_lanes() {
        let owned_tile = &*TEST_GRAPH_TILE;
        let tile = owned_tile.as_tile();

        // insta internally does a fork operation, which is not supported under Miri
        if !cfg!(miri) {
            insta::assert_debug_snapshot!(tile.turn_lanes);
        }
    }
}
