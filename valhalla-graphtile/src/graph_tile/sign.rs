use bitfield_struct::bitfield;
use zerocopy_derive::TryFromBytes;

/// Holds a generic sign with type and text.
///
/// Text is stored in the [`GraphTile`](super::GraphTile) text list,
/// and the offset is stored within the sign.
/// The directed edge index within the tile is also stored
/// so that signs can be found via either the directed edge or node index.
#[derive(TryFromBytes, Debug)]
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

impl SignType {
    const fn into_bits(self) -> u8 {
        self as _
    }
    const fn from_bits(value: u8) -> Self {
        // FIXME: This is hackish
        match value {
            0 => SignType::ExitNumber,
            1 => SignType::ExitBranch,
            2 => SignType::ExitToward,
            3 => SignType::ExitName,
            4 => SignType::GuideBranch,
            5 => SignType::GuideToward,
            6 => SignType::JunctionName,
            7 => SignType::GuidanceViewJunction,
            8 => SignType::GuidanceViewSignboard,
            9 => SignType::TollName,
            255 => SignType::Linguistic,
            _ => panic!("As far as I can tell, this crate doesn't support failable ops."),
        }
    }
}

#[bitfield(u32)]
#[derive(TryFromBytes)]
struct SignBitField {
    #[bits(22)]
    edge_or_node_index: u32,
    #[bits(8)]
    sign_type: SignType,
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
    #[inline]
    pub fn sign_type(&self) -> SignType {
        self.bitfield.sign_type()
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
