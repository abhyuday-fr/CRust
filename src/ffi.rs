use libc::{c_int, size_t};

// declare the C function to Rust
#[link(name = "telemetry")]
extern "C" {
    fn telemetry_get_frame(buf: *mut u8, buf_len: size_t) -> c_int;
}

// mirror the C constants in RUST
pub const FRAME_SIZE: usize = 18; // must match sizeof(TelemetryFrame) in C

pub const FLAG_ANTENNA_OK: u8 = 1 << 0;
pub const FLAG_GPS_LOCK: u8 = 1 << 1;
pub const FLAG_LOW_BATTERY: u8 = 1 << 2;
pub const FLAG_ERROR: u8 = 1 << 7;

// the safe wrapper
pub fn get_telemetry_frame() -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; FRAME_SIZE];

    let ret = unsafe { telemetry_get_frame(buf.as_mut_ptr(), buf.len() as size_t) };

    if ret == 0 {
        Ok(buf)
    } else {
        Err(format!("telemetry_get_frame returned error code {ret}"))
    }
}
