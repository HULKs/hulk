#pragma once

#include <chrono>
#include <cstdint>
#include <time.h>

#ifdef SIMROBOT
#include "Hardware/SimRobot/SimRobotAdapterAdapter.hpp"
#endif
#include "Storage/UniValue/UniValue.h"


/**
 * @brief getThreadTime returns the current thread time with unknown epoch
 * @return the current thread time in ns
 */
static inline std::uint64_t getThreadTime()
{
  // This only works on Linux.
#ifdef WIN32
  // TODO: Do something for windows here...
  return 0;
#elif defined __APPLE__
  // TODO: Do something for darwin here...
  return 0;
#else
  timespec ts;
  clock_gettime(CLOCK_THREAD_CPUTIME_ID, &ts);
  return ts.tv_sec * 1000000000ULL + ts.tv_nsec;
#endif
}

/**
 * @class TimePoint
 * @brief May be used to create time points for time measurement
 */
class TimePoint
{
public:
  /**
   * @brief Creates a TimePoint at time "time", assumes "time" to be time since boot in ms.
   * @param time the time in ms
   */
  TimePoint(const unsigned int time = 0)
    : creationTime_(time)
  {
  }

  /**
   * @brief Returns the time set as base for all TimePoints (boot time).
   * @return the base time in ms
   */
  static unsigned int getBaseTime()
  {
    if (!baseTime_)
    {
      getCurrentTime();
    }
    return baseTime_;
  }

  /**
   * @brief Get the current time
   * @return The current time as TimePoint
   *
   * @see TimePoint
   */
  static TimePoint getCurrentTime()
  {
    if (!baseTime_)
    {
#ifndef SIMROBOT
      auto duration = std::chrono::system_clock::now().time_since_epoch();
      baseTime_ = static_cast<unsigned int>(std::chrono::duration_cast<std::chrono::milliseconds>(duration).count())
        - 15000;
#else
      baseTime_ = SimRobotAdapterAdapter::getSimulatedTime() - 15000;
#endif
    }
#ifndef SIMROBOT
    auto duration = std::chrono::system_clock::now().time_since_epoch();
    return TimePoint(static_cast<unsigned int>(std::chrono::duration_cast<std::chrono::milliseconds>(duration).count()) - baseTime_);
#else
    return TimePoint(SimRobotAdapterAdapter::getSimulatedTime() - baseTime_);
#endif
  }

  /**
   * @brief Returns the time passed since base time (time since boot).
   * @return the time since base time in ms since boot
   */
  unsigned int getSystemTime() const
  {
    return creationTime_;
  }

  /**
   * @brief Used to calculate the time difference between 2 time points.
   * @param endPoint second time point used for calculation
   * @return the time difference between given time points in ms since boot
   */
  int operator-(const TimePoint endPoint) const
  {
    return creationTime_ - endPoint.creationTime_;
  }

  /**
   * @brief Used to subtract a time period from a time point.
   * @param period time that is subtracted from the given time point
   * @return the difference of given time point and period in ms since boot
   */
  TimePoint operator-(const int period) const
  {
    return TimePoint(creationTime_ - period);
  }

  /**
   * @brief Used to subtract a duration from a time point
   * @param duration the duration that is subtracted from the given time point
   * @return the sum of given time point and duration in ms since boot
   */
  template<typename T, typename U>
  TimePoint operator-(const std::chrono::duration<T, U>& duration) const
  {
    auto d = std::chrono::duration_cast<std::chrono::milliseconds>(duration);
    return TimePoint(creationTime_ - static_cast<int>(d.count()));
  }

  /**
   * @brief Used to add a time period to a time point.
   * @param period time that is added to the given time point
   * @return the sum of given time point and period in ms since boot
   */
  TimePoint operator+(const int period) const
  {
    return TimePoint(creationTime_ + period);
  }

  /**
   * @brief Used to add a duration to a time point
   * @param duration the duration that is added to the given time point
   * @return the sum of given time point and duration in ms
   */
  template<typename T, typename U>
  TimePoint operator+(const std::chrono::duration<T, U>& duration) const
  {
    auto d = std::chrono::duration_cast<std::chrono::milliseconds>(duration);
    return TimePoint(creationTime_ + static_cast<int>(d.count()));
  }

  /**
   * @brief Used to compare 2 time points.
   * @param endPoint second time point used for calculation
   * @return true if endPoint is smaller
   */
  bool operator>(const TimePoint endPoint) const
  {
    return (creationTime_ > endPoint.creationTime_);
  }

  /**
   * @brief Used to compare 2 time points.
   * @param endPoint second time point used for calculation
   * @return true if endPoint is larger
   */
  bool operator<(const TimePoint endPoint) const
  {
    return (creationTime_ < endPoint.creationTime_);
  }

  /**
   * @brief Used to compare 2 time points.
   * @param endPoint second time point used for calculation
   * @return true is endPoint is equal
   */
  bool operator==(const TimePoint endPoint) const
  {
    return (creationTime_ == endPoint.creationTime_);
  }

  /**
   * @brief Used to compare 2 time points.
   * @param endPoint second time point used for calculation
   * @return true is endPoint is not equal
   */
  bool operator!=(const TimePoint endPoint) const
  {
    return (creationTime_ != endPoint.creationTime_);
  }
private:
  /// The base time used for every TimePoint in ms
  static unsigned int baseTime_;
  /// The system time at which this TimePoint was created in ms
  unsigned int creationTime_;
};

/**
 * @brief Used for streaming from a TimePoint to Uni::Value
 * @param out Uni::Value variable that is going to be filled with info about TimePoint in
 * @param in TimePoint that is streamed to Uni::Value
 */
inline void operator<<(Uni::Value& out, const TimePoint in)
{
  out << in.getSystemTime();
}

/**
 * @brief Used for streaming from Uni::Value to TimePoint
 * @param in Uni::Value variable that is used to create a TimePoint
 * @param out TimePoint that is created from Uni::Value
 */
inline void operator>>(const Uni::Value& in, TimePoint& out)
{
  unsigned int i;
  in >> i;
  out = TimePoint(i);
}

/**
 * @enum TDT
 * @brief The TDT enum class is representing time difference types
 */
enum class TDT
{
  SECS,
  MILS
};

/**
 * @brief Get the time difference of two time points
 * @param lhs A TimePoint value
 * @param rhs Another TimePoint value
 * @param type A timeDiff value specifying the type if time format (TDT::SECS,
 *        TDT::MILS)
 * @return The time difference as float value
 *
 * @see getTimePoint
 * @see TDT
 */
static inline float getTimeDiff(const TimePoint lhs, const TimePoint rhs, const TDT type)
{
  unsigned int diff;
  if (lhs > rhs)
  {
    diff = lhs - rhs;
  }
  else
  {
    diff = rhs - lhs;
  }
  switch (type)
  {
    case TDT::SECS:
      return static_cast<float>(diff) / 1000.0f;
    case TDT::MILS:
      return diff;
    default:
      return diff;
  }
}
