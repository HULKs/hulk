#pragma once

#include "Framework/DebugDatabase.hpp"
#include <cstdint>
#include <string>

class Chronometer
{
public:
  /**
   * @brief Chronometer constructor - saves the current time
   * @param debug a reference to the Debug instance
   * @param key the key for the debug protocol value
   * @author Arne Hasselbring
   */
  Chronometer(DebugDatabase::DebugMap& debug, std::string key);
  Chronometer(const Chronometer&) = delete;
  Chronometer(Chronometer&&) = delete;
  Chronometer& operator=(const Chronometer&) = delete;
  Chronometer& operator=(Chronometer&&) = delete;

  /**
   * @brief Chronometer destructor - gets the current time, calculates the
   * difference to the saved time (in milliseconds) and logs it via the Debug class
   * @author Arne Hasselbring
   */
  ~Chronometer();

  /**
   * @brief stop stops the timing of this chronometer and sends the debug update
   */
  void stop();

private:
  static inline std::uint64_t getThreadTime()
  {
    timespec ts{};
    clock_gettime(CLOCK_THREAD_CPUTIME_ID, &ts);
    return ts.tv_sec * 1000000000ULL + ts.tv_nsec;
  }

  /// the key for the time value
  const std::string key_;
  /// a reference to the Debug instance
  DebugDatabase::DebugMap& debug_;
  /// the timestamp at object construction
  std::uint64_t startTime_;
  /// whether this chronometer timing was already stopped
  std::atomic_bool isStopped_{false};
};
