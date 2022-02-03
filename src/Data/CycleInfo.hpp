#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"

class CycleInfo : public DataType<CycleInfo>
{
public:
  /// the name of this DataType
  DataTypeName name__{"CycleInfo"};
  /**
   * @brief Calculates the duration between this cycle's start time and some other point in time
   * @param Clock::time_point the other time point
   * @tparam Period the period of the duration's return type
   * @return the elapsed time in the requested unit
   */
  inline Clock::duration getAbsoluteTimeDifference(const Clock::time_point timePoint) const
  {
    return std::chrono::abs(timePoint - startTime);
  }
  /// the time when the cycle started
  Clock::time_point startTime;
  /// the duration of a cycle (between the last cycle's startTime and this startTime)
  Clock::duration cycleTime;
  /// whether the content is valid
  bool valid = false;
  /**
   * @brief reset does nothing
   */
  void reset() override
  {
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["startTime"] << startTime;
    value["cycleTime"] << cycleTime;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["startTime"] >> startTime;
    value["cycleTime"] >> cycleTime;
  }
};
