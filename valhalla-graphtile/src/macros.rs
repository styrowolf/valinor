/// Transmutes variable length data into a Vec<T>.
/// This can't be written as a function because the const generics
/// require explicit types and that context isn't available from function generic params.
#[macro_export]
macro_rules! transmute_variable_length_data {
    ($type:ty, $data:expr, $offset:expr, $item_count:expr) => {{
        const PTR_SIZE: usize = size_of::<$type>();
        (0..$item_count)
            .map(|i| {
                let range = $offset + PTR_SIZE * i..$offset + PTR_SIZE * (i + 1);
                let slice: [u8; PTR_SIZE] = $data[range].try_into()?;
                Ok(transmute!(slice))
            })
            .collect::<Result<_, GraphTileError>>()
            .map(|res| (res, $offset + PTR_SIZE * $item_count))
    }};
}

/// Tries to transmute variable length data into a Vec<T>.
/// Analogous to [`transmute_variable_length_data`](crate::transmute_variable_length_data),
/// but for types implementing [`zerocopy::TryFromBytes`]
/// rather than [`zerocopy::FromBytes`].
#[macro_export]
macro_rules! try_transmute_variable_length_data {
    ($type:ty, $data:expr, $offset:expr, $item_count:expr) => {{
        const PTR_SIZE: usize = size_of::<$type>();
        (0..$item_count)
            .map(|i| {
                let range = $offset + PTR_SIZE * i..$offset + PTR_SIZE * (i + 1);
                let slice: [u8; PTR_SIZE] = $data[range].try_into()?;
                try_transmute!(slice).map_err(|_| GraphTileError::ValidityError)
            })
            .collect::<Result<_, GraphTileError>>()
            .map(|res| (res, $offset + PTR_SIZE * $item_count))
    }};
}
