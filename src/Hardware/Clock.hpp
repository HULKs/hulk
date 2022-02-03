#pragma once

#include <chrono>

using namespace std::chrono_literals;

/// Implements TrivialClock concept (without now()), represents either real time on NAO or
/// simulation time in simulators
struct Clock
{
  using rep = float;
  using period = std::chrono::seconds::period;
  using duration = std::chrono::duration<rep, period>;
  using time_point = std::chrono::time_point<Clock>;
  // The TrivialClock concept requires "is_steady" which collides with our naming convention
  // NOLINTNEXTLINE(readability-identifier-naming)
  static const bool is_steady{false};
};
