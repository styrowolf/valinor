// TODO: We don't actively use transit data structures, so these are merely correctly sized stubs for now.

use zerocopy_derive::FromBytes;

#[derive(Debug, FromBytes)]
pub struct TransitDeparture {
    _bitfield1: u64,
    _bitfield2: u64,
    _departure_times: u64,
}

#[derive(FromBytes)]
pub struct TransitStop {
    _data: u64,
}

#[derive(FromBytes)]
pub struct TransitRoute {
    _route_color: u32,
    _route_text_color: u32,

    _bitfield1: u64,
    _bitfield2: u64,
    _bitfield3: u64,
    _bitfield4: u64,
}

#[derive(FromBytes)]
pub struct TransitSchedule {
    _days: u64,
    _bitfield1: u64,
}

#[derive(FromBytes)]
pub struct TransitTransfer {
    from_stop_id: u32,
    to_stop_id: u32,
    _bitfield_1: u32,
}
