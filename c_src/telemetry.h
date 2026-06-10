#ifndef TELEMETRY_H
#define TELEMETRY_H

#include <stddef.h>
#include <stdint.h>

#pragma pack(push, 1)

typedef struct {
  uint32_t timestamp_ms;
  int16_t temperature_c;
  uint16_t battery_mv;
  uint32_t altitude_m;
  int16_t velocity_ms;
  uint8_t signal_strength; // RSSI 0-255
  uint8_t status_flags;    // bits flags
  uint16_t checksum;
} TelemetryFrame;

#pragma pack(pop)

// status flag bit masks
// test with: flags & SAT_FLAG_ANTENNA_OK
#define SAT_FLAG_ANTENNA_OK (1 << 0)  // bit 0
#define SAT_FLAG_GPS_LOCK (1 << 1)    // bit 1
#define SAT_FLAG_LOW_BATTERY (1 << 2) // bit 2
#define SAT_FLAG_ERROR (1 << 7)       // bit 7

// the size Rust needs to allocate its buffer
#define TELEMETRY_FRAME_SIZE sizeof(TelemetryFrame)

int telemetry_get_frame(uint8_t *buf, size_t buf_len);

#endif
