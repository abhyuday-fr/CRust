use crate::ffi;
use bytemuck::{Pod, Zeroable};

// Rust mirror of TelemetryFrame
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TelemetryFrame {
    pub timestamp_ms: u32,
    pub temperature_c: i16,
    pub battery_mv: u16,
    pub altitude_m: u32,
    pub velocity_ms: i16,
    pub signal_strength: u8,
    pub status_flags: u8,
    pub checksum: u16,
}

// human readable interpretation
impl TelemetryFrame {
    // Convert raw i16 (stored as 0.01°C) to f32 degrees
    pub fn temperature_celsius(&self) -> f32 {
        self.temperature_c as f32 / 100.0
    }

    // Convert raw i16 (stored as cm/s) to f32 m/s
    pub fn velocity_metres_per_sec(&self) -> f32 {
        self.velocity_ms as f32 / 100.0
    }

    // Convert millivolts to volts
    pub fn battery_volts(&self) -> f32 {
        self.battery_mv as f32 / 1000.0
    }

    // Battery percentage — 4200mV = 100%, 3000mV = 0%
    pub fn battery_percent(&self) -> u8 {
        let mv = self.battery_mv as i32;
        let pct = ((mv - 3000) * 100) / 1200;
        pct.clamp(0, 100) as u8
    }

    // Signal strength as percentage
    pub fn signal_percent(&self) -> u8 {
        // raw 0-255 → 0-100%
        ((self.signal_strength as u16 * 100) / 255) as u8
    }

    // Mission elapsed time broken into hours/minutes/seconds
    pub fn elapsed_time(&self) -> (u32, u32, u32) {
        let total_secs = self.timestamp_ms / 1000;
        let h = total_secs / 3600;
        let m = (total_secs % 3600) / 60;
        let s = total_secs % 60;
        (h, m, s)
    }

    // Verify the checksum matches — detect corrupted frames
    pub fn is_valid(&self) -> bool {
        let bytes = bytemuck::bytes_of(self);
        let xor: u8 = bytes[..bytes.len() - 2].iter().fold(0u8, |acc, &b| acc ^ b);
        // our C checksum is u16 but only uses low byte via XOR chain
        xor == (self.checksum & 0xFF) as u8
    }

    // Decode status_flags into human-readable strings
    pub fn status_summary(&self) -> Vec<&'static str> {
        let mut statuses = Vec::new();
        if self.status_flags & ffi::FLAG_ANTENNA_OK != 0 {
            statuses.push("ANTENNA OK");
        }
        if self.status_flags & ffi::FLAG_GPS_LOCK != 0 {
            statuses.push("GPS LOCK");
        }
        if self.status_flags & ffi::FLAG_LOW_BATTERY != 0 {
            statuses.push("LOW BATTERY");
        }
        if self.status_flags & ffi::FLAG_ERROR != 0 {
            statuses.push("ERROR");
        }
        statuses
    }
}

// parse raw bytes into a TelemetryFrame
pub fn parse(raw: &[u8]) -> Result<TelemetryFrame, String> {
    if raw.len() != std::mem::size_of::<TelemetryFrame>() {
        return Err(format!(
            "expected {} bytes, got {}",
            std::mem::size_of::<TelemetryFrame>(),
            raw.len()
        ));
    }
    Ok(*bytemuck::from_bytes::<TelemetryFrame>(raw))
}
