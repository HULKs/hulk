#pragma once

#include <cstdint>
#include <string>

#include "Framework/DebugDatabase.hpp"

class Chronometer
{
public:
  /**
   * @brief Chronometer constructor - saves the current time
   * @param debug a reference to the Debug instance
   * @param key the key for the debug protocol value
   * @author Arne Hasselbring
   */
  Chronometer(DebugDatabase::DebugMap& debug, const std::string& key);
  /**
   * @brief Chronometer destructor - gets the current time, calculates the
   * difference to the saved time (in milliseconds) and logs it via the Debug class
   * @author Arne Hasselbring
   */
  ~Chronometer();

private:
  /// the key for the time value
  const std::string key_;
  /// a reference to the Debug instance
  DebugDatabase::DebugMap& debug_;
  /// the timestamp at object construction
  std::uint64_t startTime_;
};
