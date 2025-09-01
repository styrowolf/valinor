///! Helpers for converting between native integers and zerocopy endian-aware helpers.

pub mod conv_u64le {
    use zerocopy::{LE, U64};
    pub const fn from_inner(n: u64) -> U64<LE> {
        U64::<LE>::new(n)
    }
    pub const fn into_inner(v: U64<LE>) -> u64 {
        v.get()
    }
}

pub mod conv_u32le {
    use zerocopy::{LE, U32};
    pub const fn from_inner(n: u32) -> U32<LE> {
        U32::<LE>::new(n)
    }
    pub const fn into_inner(v: U32<LE>) -> u32 {
        v.get()
    }
}

pub mod conv_u16le {
    use zerocopy::{LE, U16};
    pub const fn from_inner(n: u16) -> U16<LE> {
        U16::<LE>::new(n)
    }
    pub const fn into_inner(v: U16<LE>) -> u16 {
        v.get()
    }
}
