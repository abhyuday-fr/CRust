#include "telemetry.h"
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

// internal state
static uint32_t s_tick = 0; // fake clock, increments each call

// seed rand() once on first call
static void maybe_seed(void) {
  static int seeded = 0;
  if (!seeded) {
    srand((unsigned int)time(NULL));
    seeded = 1;
  }
}

static int rand_range(int min, int max) {
  return min + rand() % (max - min + 1);
}

// checksum
// XOR every byte of the frame except the checksum field itself
static uint16_t compute_checksum(const TelemetryFrame *f) {
  const uint8_t *bytes = (const uint8_t *)f;
  uint16_t xor = 0;
  for (size_t i = 0; i < sizeof(TelemetryFrame) - sizeof(uint16_t); i++) {
    xor ^= bytes[i];
  }
  return xor;
}

// public API
int telemetry_get_frame(uint8_t *buf, size_t buf_len) {
  if (buf == NULL || buf_len < sizeof(TelemetryFrame)) {
    return -1;
  }

  maybe_seed();
  s_tick++;

  TelemetryFrame frame;

  // 100 ms per tick
  frame.timestamp_ms = s_tick * 100;

  // temp around 23 C w/ +- 2 C noise
  frame.temperature_c = (int16_t)(2300 + rand_range(-200, 200));

  // battery starts at 4200 mV, drains 1 mV per tick
  uint16_t base_mv = (s_tick < 4200) ? (uint16_t)(4200 - s_tick) : 0;
  frame.battery_mv = base_mv + (uint16_t)rand_range(0, 10);

  frame.altitude_m = (uint32_t)(400000 + rand_range(-500, 500));

  frame.velocity_ms = (int16_t)rand_range(-50, 50);

  // signal degrades slightly as tick advances, with noise
  int sig = 220 - (int)(s_tick / 10) + rand_range(-10, 10);
  frame.signal_strength = (uint8_t)(sig < 0 ? 0 : sig > 255 ? 255 : sig);

  // status flags : assemble bit-by-bit
  frame.status_flags = 0;
  frame.status_flags |= SAT_FLAG_ANTENNA_OK;
  if (frame.signal_strength > 80)
    frame.status_flags |= SAT_FLAG_GPS_LOCK;
  if (frame.battery_mv < 3500)
    frame.status_flags |= SAT_FLAG_LOW_BATTERY;
  if (frame.battery_mv == 0)
    frame.status_flags |= SAT_FLAG_ERROR;

  // checksum is last
  frame.checksum = compute_checksum(&frame);

  // copying packed struct into the calller's buffer
  memcpy(buf, &frame, sizeof(TelemetryFrame));
  return 0;
}
