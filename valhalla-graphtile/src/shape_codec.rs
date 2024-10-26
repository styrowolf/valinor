//! # Shape encoding/decoding
//!
//! Valhalla uses a mix of varint and delta encoding to efficiently pack shape information.
//! This module contains various helpers for converting back and forth.
//!
//! See Google's [protobuf docs](https://protobuf.dev/programming-guides/encoding/)
//! for info on varint encoding generally.

use bytes::Bytes;
use bytes_varint::{VarIntError, VarIntSupport};
use geo::{coord, Coord, LineString};

const DECODE_PRECISION: f64 = 1e-6;

/// Decodes a Valhalla encoded shape from a byte buffer of exact size.
///
/// # Errors
///
/// Decoding may fail if the byte buffer is not exactly sized
/// (you cannot include extra information at the end of a slice).
/// It will also fail if the varint data is malformed.
pub fn decode_shape(bytes: &Bytes) -> Result<LineString<f64>, VarIntError> {
    // Pre-allocating 1/4 the byte len is an optimization from Valhalla.
    // Sounds about right since we can expect most values to be small due to delta encoding.
    let mut coords: Vec<Coord> = Vec::with_capacity(bytes.len() / 4);
    let mut bytes = bytes.clone();
    let mut lat = 0;
    let mut lon = 0;
    while !bytes.is_empty() {
        lat += bytes.try_get_i32_varint()?;
        lon += bytes.try_get_i32_varint()?;
        coords.push(coord! {
            x: f64::from(lon) * DECODE_PRECISION,
            y: f64::from(lat) * DECODE_PRECISION,
        });
    }
    Ok(coords.into())
}
