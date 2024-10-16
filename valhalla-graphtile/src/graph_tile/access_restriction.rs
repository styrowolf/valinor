use crate::graph_tile::GraphTileError;
use crate::Access;
use bitfield_struct::bitfield;
use enumset::EnumSet;
use zerocopy::try_transmute;
use zerocopy_derive::{FromBytes, TryFromBytes};

/// Types of access restrictions.
#[derive(TryFromBytes, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum AccessRestrictionType {
    Hazmat,
    MaxHeight,
    MaxWidth,
    MaxLength,
    MaxWeight,
    MaxAxleLoad,
    TimedAllowed,
    TimedDenied,
    DestinationAllowed,
    MaxAxles,
    // Valhalla docs say the max supported value is 31,
    // but the field looks 6 bits wide to me, not 5.
}

#[bitfield(u64)]
#[derive(PartialEq, Eq, FromBytes)]
struct AccessRestrictionBitField {
    #[bits(22)]
    edge_index: u32,
    #[bits(6)]
    restriction_type: u8,
    #[bits(12)]
    modes: u16,
    #[bits(24)]
    _spare: u32,
}

/// Access restrictions beyond the usual access tags
#[derive(PartialEq, Eq, TryFromBytes, Debug)]
#[repr(C)]
pub struct AccessRestriction {
    bitfield: AccessRestrictionBitField,
    value: u64,
}

impl AccessRestriction {
    /// Gets the edge index (within the tile) to which the restriction applies.
    #[inline]
    pub fn edge_index(&self) -> u32 {
        self.bitfield.edge_index()
    }

    /// Gets the type of access restriction.
    ///
    /// # Panics
    ///
    /// This function currently panics if the edge use contains an invalid bit pattern.
    /// We should check the validity of the field when we do the initial try_transmute
    /// on the type (and then have an error result that the tile is invalid).
    /// There are some upstream bugs though between zerocopy and bitfield-struct
    /// macros: https://github.com/google/zerocopy/issues/388.
    #[inline]
    pub fn restriction_type(&self) -> AccessRestrictionType {
        try_transmute!(self.bitfield.restriction_type())
            .map_err(|_| GraphTileError::ValidityError)
            .expect("Invalid bit pattern for restriction type.")
    }

    /// The access modes affected by this restriction.
    #[inline]
    pub fn affected_access_modes(&self) -> EnumSet<Access> {
        // TODO: Look at ways to do this with FromBytes; this currently copies
        // Safety: The access bits are length 12, so invalid representations are impossible.
        unsafe { EnumSet::from_repr_unchecked(self.bitfield.modes()) }
    }
}

#[cfg(test)]
mod tests {
    use super::{Access, AccessRestrictionType};
    use crate::graph_tile::TEST_GRAPH_TILE;
    use enumset::EnumSet;

    #[test]
    fn test_parse_access_restrictions_count() {
        let tile = &*TEST_GRAPH_TILE;

        assert_eq!(
            tile.access_restrictions.len(),
            tile.header.access_restriction_count() as usize
        );
    }

    #[test]
    fn test_parse_access_restrictions() {
        let tile = &*TEST_GRAPH_TILE;

        insta::assert_debug_snapshot!(tile.access_restrictions[0]);

        assert_eq!(
            tile.access_restrictions[0].restriction_type(),
            AccessRestrictionType::MaxWeight
        );
        assert_eq!(
            tile.access_restrictions[0].affected_access_modes(),
            EnumSet::from(Access::Truck)
        );

        insta::assert_debug_snapshot!(tile.access_restrictions.last().unwrap());

        // TODO: Other sanity checks after we add some more advanced methods
    }
}
