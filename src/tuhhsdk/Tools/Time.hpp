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
  explicit TimePoint(const uint32_t time = 0)
    : creationTime_(time)
  {
  }

  /**
   * @brief Copy constructor copies another TimePoint object
   * @param other the TimePoint to copy
   */
  TimePoint(const TimePoint& other) = default;

  /**
   * @brief Returns the time set as base for all TimePoints (boot time).
   * @return the base time in ms
   */
  static uint64_t getBaseTime()
  {
#ifndef SIMROBOT
    return static_cast<uint64_t>(
        std::chrono::duration_cast<std::chrono::milliseconds>(baseTime_.time_since_epoch())
            .count());
#else
    if (!baseTime_)
    {
      getCurrentTime();
    }
    return static_cast<uint64_t>(baseTime_);
#endif
  }

  /**
   * @brief Get the current time
   * @return The current time as TimePoint
   *
   * @see TimePoint
   */
  static TimePoint getCurrentTime()
  {
#if defined(NAOV6)
    auto duration = std::chrono::steady_clock::now() - baseTime_;
    return TimePoint(static_cast<uint32_t>(
        std::chrono::duration_cast<std::chrono::milliseconds>(duration).count()));
#elif defined(NAOV5) || defined(REPLAY)
    auto duration = std::chrono::system_clock::now() - baseTime_;
    return TimePoint(static_cast<uint32_t>(
        std::chrono::duration_cast<std::chrono::milliseconds>(duration).count()));
#else // Simrobot
    if (baseTime_ == 0)
    {
      baseTime_ = SimRobotAdapterAdapter::getSimulatedTime();
    }
    return TimePoint(SimRobotAdapterAdapter::getSimulatedTime() - baseTime_);
#endif
  }

  /**
   * @brief Returns the time passed since base time (time since boot).
   * @return the time since base time in ms since boot
   */
  uint32_t getSystemTime() const
  {
    return creationTime_;
  }

  /**
   * @brief default assignment operator
   * @param other the other object to assign
   * @return reference to the the timepoint object
   */
  TimePoint& operator=(const TimePoint& other) = default;

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
  template <typename T, typename U>
  TimePoint operator-(const std::chrono::duration<T, U>& duration) const
  {
    auto d = std::chrono::duration_cast<std::chrono::milliseconds>(duration);
    return TimePoint(creationTime_ - static_cast<uint32_t>(d.count()));
  }

  /**
   * @brief Used to subtract and assign a duration to/from a time point
   * @param subtrahend the duration to be subtracted
   * @return the difference
   */
  TimePoint operator-=(const int subtrahend)
  {
    creationTime_ -= subtrahend;
    return TimePoint(creationTime_);
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
  template <typename T, typename U>
  TimePoint operator+(const std::chrono::duration<T, U>& duration) const
  {
    auto d = std::chrono::duration_cast<std::chrono::milliseconds>(duration);
    return TimePoint(creationTime_ + static_cast<int>(d.count()));
  }

  /**
   * @brief Used to add and assign a duration to a time point
   * @param summand the duration to be added
   * @return the sum
   */
  TimePoint operator+=(const int summand)
  {
    creationTime_ += summand;
    return TimePoint(creationTime_);
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
   * @return true if endPoint is smaller or equal
   */
  bool operator>=(const TimePoint endPoint) const
  {
    return (creationTime_ >= endPoint.creationTime_);
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
   * @return true if endPoint is larger or equal
   */
  bool operator<=(const TimePoint endPoint) const
  {
    return (creationTime_ <= endPoint.creationTime_);
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
  // baseTime_ is the moment when the first TimePoint was created (this happens as one of the first
  // things when our software is started). As all systems (v5, v6, simrobot) use different
  // timestamps when it comes to camera images (timestamps are determined by kernel), we decided to
  // handle the base time like the camera image time on every single system. Therefore v5 is using a
  // chrono system clock timestamp, v6 is using a steady clock time stamp and simrobot uses a
  // unsigned int (simulation time starts at 0).
#if defined(NAOV6)
  /// The base time used for every TimePoint as a chrono steady clock
  static std::chrono::time_point<std::chrono::steady_clock> baseTime_;
#elif defined(NAOV5) || defined(REPLAY)
  /// The base time used for every TimePoint as a chrono system clock
  static std::chrono::time_point<std::chrono::system_clock> baseTime_;
#else
  /// The base time used for every TimePoint in ms
  static uint32_t baseTime_;
#endif

  /// The system time at which this TimePoint was created in ms
  uint32_t creationTime_;
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
  uint32_t diff;
  if (lhs > rhs)
  {
    diff = static_cast<uint32_t>(lhs - rhs);
  }
  else
  {
    diff = static_cast<uint32_t>(rhs - lhs);
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
