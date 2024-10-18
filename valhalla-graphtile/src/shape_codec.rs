//! # Shape encoding/decoding
//!
//! Valhalla uses a mix of varint and delta encoding to efficiently pack shape information.
//! This module contains various helpers for converting back and forth.
//!
//! See Google's [protobuf docs](https://protobuf.dev/programming-guides/encoding/)
//! for info on varint encoding generally.

use bytes::Bytes;
use bytes_varint::*;
use geo::{coord, Coord, LineString};

const DECODE_PRECISION: f64 = 1e-6;

pub fn decode_shape(bytes: Bytes) -> Result<LineString<f64>, VarIntError> {
    // Pre-allocating 1/4 the byte len is an optimization from Valhalla.
    // Sounds about right since we can expect most values to be small due to delta encoding.
    let mut coords: Vec<Coord> = Vec::with_capacity(bytes.len() / 4);
    let mut bytes = bytes.clone();
    let mut lat = 0;
    let mut lon = 0;
    while !bytes.is_empty() {
        lat += bytes.get_i32_varint()?;
        lon += bytes.get_i32_varint()?;
        coords.push(coord! {
            x: f64::from(lon) * DECODE_PRECISION,
            y: f64::from(lat) * DECODE_PRECISION,
        })
    }
    Ok(coords.into())
}
