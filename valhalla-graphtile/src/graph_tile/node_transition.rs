use bitfield_struct::bitfield;
use zerocopy_derive::FromBytes;
use crate::GraphId;

#[derive(FromBytes)]
#[bitfield(u64)]
pub struct NodeTransition {
    #[bits(46)]
    end_node_id: u64,
    // Booleans represented this way for infailability.
    // See comment in node_info.rs for details.
    #[bits(1)]
    up: u8,
    #[bits(17)]
    _spare: u32,
}

impl NodeTransition {
    /// The ID of the corresponding end node on another hierarchy level.
    #[inline]
    pub fn corresponding_end_node_id(&self) -> GraphId {
        // Safety: We know that this value cannot be larger than 46 bits.
        unsafe { GraphId::from_id_unchecked(self.end_node_id()) }
    }

    /// Is the transition up to a higher level?
    /// (If not, there's only one way to go...)
    #[inline]
    pub const fn is_up(&self) -> bool {
        self.up() != 0
    }
}

#[cfg(test)]
mod test {
    use crate::graph_tile::{TEST_GRAPH_TILE};

    #[test]
    fn test_parse_transitions_count() {
        let tile = &*TEST_GRAPH_TILE;

        assert_eq!(tile.transitions.len(), tile.header.transition_count() as usize);
    }

    #[test]
    fn test_parse_transitions() {
        let tile = &*TEST_GRAPH_TILE;

        insta::assert_debug_snapshot!(tile.transitions[0]);
        insta::assert_debug_snapshot!(tile.transitions.last().unwrap());
    }
}