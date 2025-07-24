// --- Translated from C++: baldr/predictedspeeds.h ---
use std::sync::OnceLock;
use std::f32::consts::PI;
use base64::{prelude::BASE64_STANDARD, Engine};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

pub const K_COEFFICIENT_COUNT: usize = 200;
pub const K_BUCKETS_PER_WEEK: usize = (7 * 24 * 60) / K_SPEED_BUCKET_SIZE_MINUTES;
pub const K_SPEED_BUCKET_SIZE_MINUTES: usize = 5;
pub const K1_OVER_SQRT2: f32 = 0.707106781;
pub const K_PI_BUCKET_CONSTANT: f32 = PI / 2016.0;
pub const K_SPEED_NORMALIZATION: f32 = 0.031497039;
pub const K_COS_BUCKET_TABLE_SIZE: usize = K_COEFFICIENT_COUNT * K_BUCKETS_PER_WEEK;

pub struct BucketCosTable {
    table: Vec<f32> //[f32; K_COS_BUCKET_TABLE_SIZE],
}

impl BucketCosTable {
    pub fn get_instance() -> &'static BucketCosTable {
        static INSTANCE: OnceLock<BucketCosTable> = OnceLock::new();
        INSTANCE.get_or_init(|| {
            let mut table = vec![0.0f32; K_COS_BUCKET_TABLE_SIZE];
            let mut idx = 0;
            for bucket in 0..K_BUCKETS_PER_WEEK {
                for c in 0..K_COEFFICIENT_COUNT {
                    table[idx] = (K_PI_BUCKET_CONSTANT * ((bucket as f32) + 0.5) * (c as f32)).cos();
                    idx += 1;
                }
            }
            BucketCosTable { table }
        })
    }

    pub fn get(&self, bucket: usize) -> &[f32] {
        let start = bucket * K_COEFFICIENT_COUNT;
        &self.table[start..start + K_COEFFICIENT_COUNT]
    }
}

pub fn compress_speed_buckets(speeds: &[f32]) -> Vec<i16> {
    assert!(speeds.len() == K_BUCKETS_PER_WEEK);

    let mut coefficients = vec![0.0f32; K_COEFFICIENT_COUNT];
    for bucket in 0..K_BUCKETS_PER_WEEK {
        let cos_values = BucketCosTable::get_instance().get(bucket);
        for c in 0..K_COEFFICIENT_COUNT {
            coefficients[c] += cos_values[c] * speeds[bucket];
        }
    }
    coefficients[0] *= K1_OVER_SQRT2;
    let mut result = vec![0i16; K_COEFFICIENT_COUNT];
    for (i, &coef) in coefficients.iter().enumerate() {
        result[i] = (K_SPEED_NORMALIZATION * coef).round() as i16;
    }
    result
}

pub fn decompress_speed_bucket(coefficients: &[i16], bucket_idx: usize) -> f32 {
    assert!(coefficients.len() == K_COEFFICIENT_COUNT);

    let b = BucketCosTable::get_instance().get(bucket_idx);
    let mut speed = coefficients[0] as f32 * K1_OVER_SQRT2;
    for i in 1..K_COEFFICIENT_COUNT {
        speed += coefficients[i] as f32 * b[i];
    }
    speed * K_SPEED_NORMALIZATION
}

pub fn encode_compressed_speeds(coefficients: &[i16]) -> String {
    assert!(coefficients.len() == K_COEFFICIENT_COUNT);
    let mut bytes = Vec::with_capacity(K_COEFFICIENT_COUNT * 2);
    for &coef in coefficients.iter() {
        bytes.write_i16::<BigEndian>(coef).unwrap();
    }
    BASE64_STANDARD.encode(&bytes)  
}

pub fn decode_compressed_speeds(encoded: &str) -> Result<Vec<i16>, String> {
    let decoded = BASE64_STANDARD.decode(encoded).map_err(|e| e.to_string())?;
    if decoded.len() != K_COEFFICIENT_COUNT * 2 {
        return Err(format!("Decoded speed string size expected= {} actual= {}", K_COEFFICIENT_COUNT * 2, decoded.len()));
    }
    let mut coefficients = vec![0i16; K_COEFFICIENT_COUNT];
    let mut rdr = std::io::Cursor::new(decoded);
    for i in 0..K_COEFFICIENT_COUNT {
        coefficients[i] = rdr.read_i16::<BigEndian>().unwrap();
    }
    Ok(coefficients)
}

// --- End of translation ---