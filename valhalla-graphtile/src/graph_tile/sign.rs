use crate::graph_tile::GraphTileError;
use bitfield_struct::bitfield;
use zerocopy::try_transmute;
use zerocopy_derive::{FromBytes, TryFromBytes};

/// Holds a generic sign with type and text.
///
/// Text is stored in the [`GraphTile`](super::GraphTile) text list,
/// and the offset is stored within the sign.
/// The directed edge index within the tile is also stored
/// so that signs can be found via either the directed edge or node index.
#[derive(TryFromBytes)]
#[repr(u8)]
pub enum SignType {
    ExitNumber,
    ExitBranch,
    ExitToward,
    ExitName,
    GuideBranch,
    GuideToward,
    JunctionName,
    GuidanceViewJunction,
    GuidanceViewSignboard,
    TollName,
    Linguistic = 255,
}

#[derive(FromBytes)]
#[bitfield(u32)]
struct SignBitField {
    #[bits(22)]
    edge_or_node_index: u32,
    #[bits(8)]
    sign_type: u8,
    #[bits(1)]
    is_route_num_type: u8,
    #[bits(1)]
    is_text_tagged: u8,
}

#[derive(TryFromBytes)]
#[repr(C)]
pub struct Sign {
    bitfield: SignBitField,
    /// The offset into the [`GraphTile`](super::GraphTile) text list
    /// which contains the text for this sign.
    pub text_offset: u32,
}

impl Sign {
    /// Gets the index (within the same tile) of the directed edge or node this sign applies to.
    #[inline]
    pub fn edge_or_node_index(&self) -> u32 {
        self.bitfield.edge_or_node_index()
    }

    /// Gets the type of the sign.
    /// The index (within the same tile) of the directed edge or node this sign applies to.
    ///
    /// # Panics
    ///
    /// This function currently panics if the sign type contains an invalid bit pattern.
    /// We should check the validity of the field when we do the initial try_transmute
    /// on the type (and then have an error result that the tile is invalid).
    /// There are some upstream bugs though between zerocopy and bitfield-struct
    /// macros: https://github.com/google/zerocopy/issues/388.
    #[inline]
    pub fn sign_type(&self) -> SignType {
        try_transmute!(self.bitfield.sign_type())
            .map_err(|_| GraphTileError::ValidityError)
            .expect("Invalid bit pattern for restriction type.")
    }

    /// Does this sign indicate a route number, phoneme for a node, or the guidance view type?
    ///
    /// This description copied straight from Valhalla.
    /// The description and naming have room for improvement.
    #[inline]
    pub const fn is_route_num_type(&self) -> bool {
        self.bitfield.is_route_num_type() != 0
    }

    /// Is the sign text tagged?
    #[inline]
    pub const fn is_text_tagged(&self) -> bool {
        self.bitfield.is_text_tagged() != 0
    }
}

#[cfg(test)]
mod tests {
    use crate::graph_tile::TEST_GRAPH_TILE;

    #[test]
    fn test_parse_sign_count() {
        let tile = &*TEST_GRAPH_TILE;

        assert_eq!(tile.signs.len(), tile.header.sign_count() as usize);
    }

    // TODO: There aren't any signs in the test tile
}
