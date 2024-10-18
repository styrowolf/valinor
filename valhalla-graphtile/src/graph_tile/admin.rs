use crate::AsCowStr;
use std::borrow::Cow;
use zerocopy_derive::FromBytes;

#[derive(FromBytes, Debug)]
#[repr(C)]
pub struct Admin {
    /// The offset into the [`GraphTile`](super::GraphTile) text list for the country name.
    pub country_name_offset: u32,
    /// The offset into the [`GraphTile`](super::GraphTile) text list for the principal subdivision name.
    pub principal_subdivision_offset: u32,
    country_iso: [u8; 2],
    principal_subdivision_iso: [u8; 3],
    // TODO: Do we need 3 bytes for alignment?
}

impl Admin {
    /// Gets the ISO 3166-1 country code
    pub fn country_iso(&self) -> Cow<'_, str> {
        self.country_iso.as_cow_str()
    }

    /// Gets the ISO 3166-2 principal subdivision code
    pub fn principal_subdivision_iso(&self) -> Cow<'_, str> {
        self.principal_subdivision_iso.as_cow_str()
    }
}

#[cfg(test)]
mod tests {
    use crate::graph_tile::TEST_GRAPH_TILE;

    #[test]
    fn test_parse_admin_count() {
        let tile = &*TEST_GRAPH_TILE;

        assert_eq!(tile.admins.len(), tile.header.admin_count() as usize);
    }

    #[test]
    fn test_parse_admins() {
        let tile = &*TEST_GRAPH_TILE;

        insta::assert_debug_snapshot!(tile.admins);
        insta::assert_debug_snapshot!(tile
            .admins
            .iter()
            .map(|admin| admin.country_iso())
            .collect::<Vec<_>>());
        insta::assert_debug_snapshot!(tile
            .admins
            .iter()
            .map(|admin| admin.principal_subdivision_iso())
            .collect::<Vec<_>>());
    }
}
