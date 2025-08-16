use crate::Access;
use bitfield_struct::bitfield;
use enumset::EnumSet;
use zerocopy_derive::TryFromBytes;

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

impl AccessRestrictionType {
    const fn into_bits(self) -> u8 {
        self as _
    }
    const fn from_bits(value: u8) -> Self {
        // FIXME: This is hackish
        match value {
            0 => AccessRestrictionType::Hazmat,
            1 => AccessRestrictionType::MaxHeight,
            2 => AccessRestrictionType::MaxWidth,
            3 => AccessRestrictionType::MaxLength,
            4 => AccessRestrictionType::MaxWeight,
            5 => AccessRestrictionType::MaxAxleLoad,
            6 => AccessRestrictionType::TimedAllowed,
            7 => AccessRestrictionType::TimedDenied,
            8 => AccessRestrictionType::DestinationAllowed,
            9 => AccessRestrictionType::MaxAxles,
            _ => panic!(
                "Invalid AccessRestriction. As far as I can tell, this crate doesn't support failable ops."
            ),
        }
    }
}

#[bitfield(u64)]
#[derive(PartialEq, Eq, TryFromBytes)]
struct AccessRestrictionBitField {
    #[bits(22)]
    edge_index: u32,
    #[bits(6)]
    restriction_type: AccessRestrictionType,
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
    #[inline]
    pub fn restriction_type(&self) -> AccessRestrictionType {
        self.bitfield.restriction_type()
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

        insta::assert_debug_snapshot!("first_access_restriction", tile.access_restrictions[0]);

        assert_eq!(
            tile.access_restrictions[0].restriction_type(),
            AccessRestrictionType::MaxWeight
        );
        assert_eq!(
            tile.access_restrictions[0].affected_access_modes(),
            EnumSet::from(Access::Truck)
        );

        insta::assert_debug_snapshot!(
            "last_access_restriction",
            tile.access_restrictions.last().unwrap()
        );

        // TODO: Other sanity checks after we add some more advanced methods
    }
}
