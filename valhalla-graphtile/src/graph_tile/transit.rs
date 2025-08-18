//! TODO: We don't actively use transit data structures, so these are merely correctly sized stubs for now.

use zerocopy::{LE, U32, U64};
use zerocopy_derive::{FromBytes, Immutable, Unaligned};

#[derive(FromBytes, Immutable, Unaligned, Debug)]
#[repr(C)]
pub struct TransitDeparture {
    _bitfield1: U64<LE>,
    _bitfield2: U64<LE>,
    _departure_times: U64<LE>,
}

#[derive(FromBytes, Immutable, Unaligned, Debug)]
#[repr(C)]
pub struct TransitStop {
    _data: U64<LE>,
}

#[derive(FromBytes, Immutable, Unaligned, Debug)]
#[repr(C)]
pub struct TransitRoute {
    _route_color: U32<LE>,
    _route_text_color: U32<LE>,

    _bitfield1: U64<LE>,
    _bitfield2: U64<LE>,
    _bitfield3: U64<LE>,
    _bitfield4: U64<LE>,
}

#[derive(FromBytes, Immutable, Unaligned, Debug)]
#[repr(C)]
pub struct TransitSchedule {
    _days: U64<LE>,
    _bitfield1: U64<LE>,
}

#[derive(FromBytes, Immutable, Unaligned, Debug)]
#[repr(C)]
pub struct TransitTransfer {
    from_stop_id: U32<LE>,
    to_stop_id: U32<LE>,
    _bitfield_1: U32<LE>,
}
