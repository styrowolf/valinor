/// Transmutes variable length data into a &[T].
/// This can't be written as a function because the const generics
/// require explicit types and that context isn't available from function generic params.
macro_rules! transmute_slice {
    ($type:ty, $data:expr, $offset:expr, $item_count:expr) => {{
        const PTR_SIZE: usize = size_of::<$type>();
        if $data.len() < PTR_SIZE * $item_count {
            Err(GraphTileError::SliceLength)
        } else {
            let start = unsafe { $data.as_ptr().add($offset) as *const $type };
            let elements = unsafe { std::slice::from_raw_parts(start, $item_count) };
            Ok((elements, $offset + PTR_SIZE * $item_count))
        }
    }};
}
