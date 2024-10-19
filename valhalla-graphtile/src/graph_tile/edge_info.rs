use crate::{
    graph_tile::GraphTileError, shape_codec::decode_shape, transmute_variable_length_data,
};
use bitfield_struct::bitfield;
use bytes::Bytes;
use bytes_varint::VarIntError;
use geo::LineString;
use std::cell::OnceCell;
use zerocopy::transmute;
use zerocopy_derive::FromBytes;

#[derive(FromBytes)]
#[bitfield(u32)]
pub struct NameInfo {
    #[bits(24)]
    name_offset: u32,
    #[bits(4)]
    additional_fields: u8,
    #[bits(1)]
    is_route_num: u8,
    #[bits(1)]
    is_tagged: u8,
    #[bits(2)]
    _spare: u8,
}

#[derive(FromBytes)]
#[bitfield(u32)]
struct FirstInnerBitfield {
    #[bits(12)]
    mean_elevation: u16,
    #[bits(4)]
    bike_network: u8,
    #[bits(8)]
    speed_limit: u8,
    #[bits(8)]
    extended_way_id: u8,
}

#[derive(FromBytes)]
#[bitfield(u32)]
struct SecondInnerBitfield {
    #[bits(4)]
    name_count: u8,
    #[bits(16)]
    encoded_shape_size: u16,
    #[bits(8)]
    extended_way_id: u8,
    #[bits(2)]
    extended_way_id_size: u8,
    #[bits(1)]
    has_elevation: u8,
    #[bits(1)]
    _spare: u8,
}

#[derive(Debug, FromBytes)]
#[repr(C)]
struct EdgeInfoInner {
    // The first part of the OSM way ID
    // TODO: would be nice to reshuffle this in v4.0
    way_id: u32,
    first_inner_bitfield: FirstInnerBitfield,
    second_inner_bitfield: SecondInnerBitfield,
}

#[derive(Debug)]
#[repr(C)]
pub struct EdgeInfo {
    inner: EdgeInfoInner,
    name_info_list: Vec<NameInfo>,
    /// The raw varint-encoded shape bytes.
    pub encoded_shape: Bytes,
    decoded_shape: OnceCell<LineString<f64>>,
    // TODO: Final 2 bytes of a 64-bit way ID
    // TODO: Encoded elevation (pointer?)
    // TODO: name list (pointer?)
    // TODO: name list length (oh shit that's a size_t...)
    // TODO: Tag cache
}

impl EdgeInfo {
    /// Gets the shape of the edge geometry as a LineString.
    ///
    /// # Performance
    ///
    /// If this [`EdgeInfo`] geometry has not been accessed previously,
    /// then it will be decoded during this method call.
    /// Subsequent fetches will be cached for as long as the `EdgeInfo`
    /// is live.
    pub fn shape(&self) -> Result<&LineString<f64>, VarIntError> {
        // TODO: Use https://doc.rust-lang.org/core/cell/struct.OnceCell.html#method.get_or_try_init when stabilized
        match self.decoded_shape.get() {
            Some(linestring) => Ok(linestring),
            None => {
                let shape = decode_shape(self.encoded_shape.clone())?;
                Ok(self.decoded_shape.get_or_init(|| shape))
            }
        }
    }
}

// TODO: Feels like this could be a macro
impl TryFrom<Bytes> for EdgeInfo {
    type Error = GraphTileError;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        let value = &bytes;
        const INNER_SIZE: usize = size_of::<EdgeInfoInner>();
        let inner_slice: [u8; INNER_SIZE] = (&value[0..INNER_SIZE]).try_into()?;
        let inner: EdgeInfoInner = transmute!(inner_slice);

        let (name_info_list, offset) = transmute_variable_length_data!(
            NameInfo,
            value,
            INNER_SIZE,
            inner.second_inner_bitfield.name_count() as usize
        )?;

        let encoded_shape =
            bytes.slice(offset..offset + inner.second_inner_bitfield.encoded_shape_size() as usize);

        Ok(Self {
            inner,
            name_info_list,
            encoded_shape,
            decoded_shape: OnceCell::new(),
        })
    }
}
