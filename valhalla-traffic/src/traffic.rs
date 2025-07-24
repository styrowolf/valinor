
pub const TRAFFIC_TILE_VERSION: u8 = 3;

use zerocopy_derive::{FromBytes, Immutable, IntoBytes, KnownLayout};

#[derive(IntoBytes, Immutable, FromBytes, KnownLayout)]
#[repr(C)]
pub struct TrafficTileHeader {
    pub tile_id: u64,
    pub last_update: u64, // seconds since epoch
    pub directed_edge_count: u32,
    pub traffic_tile_version: u32,
    pub spare2: u32,
    pub spare3: u32,
}

// Traffic speed encoding constants
pub const UNKNOWN_TRAFFIC_SPEED_RAW: u32 = (1 << 7) - 1;
pub const MAX_TRAFFIC_SPEED_KPH: u32 = (UNKNOWN_TRAFFIC_SPEED_RAW - 1) << 1;
pub const UNKNOWN_TRAFFIC_SPEED_KPH: u32 = UNKNOWN_TRAFFIC_SPEED_RAW << 1;

pub const UNKNOWN_CONGESTION_VAL: u8 = 0;
pub const MAX_CONGESTION_VAL: u8 = 63;

use bitfield_struct::bitfield;

#[bitfield(u64)]
#[derive(PartialEq, Eq, IntoBytes, Immutable, FromBytes)]
pub struct TrafficSpeed {
    #[bits(7)]
    overall_encoded_speed: u8, // 0-255kph in 2kph resolution 
    #[bits(7)]
    encoded_speed1: u8, // 0-255kph in 2kph resolution 
    #[bits(7)]
    encoded_speed2: u8, // 0-255kph in 2kph resolution 
    #[bits(7)]
    encoded_speed3: u8, // 0-255kph in 2kph resolution 
    #[bits(8)]
    breakpoint1: u8, // position = length * (breakpoint1 / 255)
    #[bits(8)]
    breakpoint2: u8, // position = length * (breakpoint2 / 255)
    #[bits(6)]
    congestion1: u8, // Stores 0 (unknown), or 1->63 (no congestion->max congestion)
    #[bits(6)]
    congestion2: u8,
    #[bits(6)]
    congestion3: u8,
    #[bits(1)]
    has_incidents: bool,  // Are there incidents on this edge in the corresponding incident tile
    #[bits(1)]
    spare: bool,
}

impl TrafficSpeed {
    pub fn with_values(
        overall_encoded_speed: u8,
        encoded_speed1: u8,
        encoded_speed2: u8,
        encoded_speed3: u8,
        breakpoint1: u8,
        breakpoint2: u8,
        congestion1: u8,
        congestion2: u8,
        congestion3: u8,
        has_incidents: bool,
    ) -> Self {
        TrafficSpeed::new()
            .with_overall_encoded_speed(overall_encoded_speed >> 1)
            .with_encoded_speed1(encoded_speed1 >> 1)
            .with_encoded_speed2(encoded_speed2 >> 1)
            .with_encoded_speed3(encoded_speed3 >> 1)
            .with_breakpoint1(breakpoint1)
            .with_breakpoint2(breakpoint2)
            .with_congestion1(congestion1)
            .with_congestion2(congestion2)
            .with_congestion3(congestion3)
            .with_has_incidents(has_incidents)
    }

    pub fn speed_valid(&self) -> bool {
        self.breakpoint1() != 0 && self.overall_encoded_speed() != UNKNOWN_TRAFFIC_SPEED_RAW as u8
    }

    pub fn closed(&self) -> bool {
        self.breakpoint1() != 0 && self.overall_encoded_speed() == 0
    }

    pub fn closed_subsegment(&self, subsegment: usize) -> bool {
        if !self.speed_valid() { return false; }
        match subsegment {
            0 => self.encoded_speed1() == 0 || self.congestion1() == MAX_CONGESTION_VAL,
            1 => self.breakpoint1() < 255 && (self.encoded_speed2() == 0 || self.congestion2() == MAX_CONGESTION_VAL),
            2 => self.breakpoint2() < 255 && (self.encoded_speed3() == 0 || self.congestion3() == MAX_CONGESTION_VAL),
            _ => panic!("Bad subsegment"),
        }
    }

    pub fn get_overall_speed(&self) -> u8 {
        self.overall_encoded_speed() << 1
    }

    pub fn get_speed(&self, subsegment: usize) -> u8 {
        if !self.speed_valid() {
            return UNKNOWN_TRAFFIC_SPEED_KPH as u8;
        }
        match subsegment {
            0 => self.encoded_speed1() << 1,
            1 => self.encoded_speed2() << 1,
            2 => self.encoded_speed3() << 1,
            _ => panic!("Bad subsegment"),
        }
    }
}