//! # Shape encoding/decoding
//!
//! Valhalla uses a mix of varint and delta encoding to efficiently pack shape information.
//! This module contains various helpers for converting back and forth.
//!
//! See Google's [protobuf docs](https://protobuf.dev/programming-guides/encoding/)
//! for info on varint encoding generally.

use geo::{Coord, CoordFloat, coord};
use integer_encoding::VarIntReader;
use num_traits::FromPrimitive;

const DECODE_PRECISION: f64 = 1e-6;

/// Decodes a Valhalla encoded shape from a byte buffer of exact size.
///
/// # Errors
///
/// Decoding may fail if the byte buffer is not exactly sized
/// (you cannot include extra information at the end of a slice).
/// It will also fail if the varint data is malformed.
pub fn decode_shape<T: CoordFloat + FromPrimitive>(bytes: &[u8]) -> std::io::Result<Vec<Coord<T>>> {
    // Pre-allocating 1/4 the byte len is an optimization from Valhalla.
    // This sounds about right since we can expect most values to be small due to delta encoding.
    let mut coords: Vec<Coord<T>> = Vec::with_capacity(bytes.len() / 4);

    // Can't fail
    let prec = T::from(DECODE_PRECISION).expect("Conversion from i32 to float should not fail");

    let mut lat = 0;
    let mut lon = 0;
    let mut bytes = bytes;
    while !bytes.is_empty() {
        lat += bytes.read_varint::<i32>()?;
        lon += bytes.read_varint::<i32>()?;
        coords.push(coord! {
            x: T::from(lon).expect("Conversion from i32 to float should not fail") * prec,
            y: T::from(lat).expect("Conversion from i32 to float should not fail") * prec,
        });
    }

    Ok(coords)
}

/// Decodes the _first_ coordinate in a Valhalla encoded shape from a byte buffer.
///
/// # Errors
///
/// Decoding may fail if the byte buffer is incorrectly sized or corrupt.
pub fn decode_first_coordinate<T: CoordFloat + FromPrimitive>(
    bytes: &[u8],
) -> std::io::Result<Coord<T>> {
    let mut bytes = bytes;

    let prec = T::from(DECODE_PRECISION).expect("Conversion from i32 to float should not fail");

    let lat = bytes.read_varint::<i32>()?;
    let lon = bytes.read_varint::<i32>()?;

    Ok(coord! {
        x: T::from(lon).expect("Conversion from i32 to float should not fail") * prec,
        y: T::from(lat).expect("Conversion from i32 to float should not fail") * prec,
    })
}
