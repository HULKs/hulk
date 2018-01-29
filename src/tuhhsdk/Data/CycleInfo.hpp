#pragma once

#include "Tools/Time.hpp"
#include "Framework/DataType.hpp"


class CycleInfo : public DataType<CycleInfo>
{
public:
  /**
   * @brief getTimeDiff calculates the time difference from this cycle to some other time point
   * @param rhs the other time point
   * @param type the unit in which the time difference should be returned
   * @return the elapsed time in the requested unit
   */
  float getTimeDiff(const TimePoint rhs, const TDT type = TDT::SECS) const
  {
    return ::getTimeDiff(startTime, rhs, type);
  }
  /// the time when the cycle started
  TimePoint startTime;
  /// the duration of a cycle [s]
  float cycleTime;
  /**
   * @brief reset does nothing
   */
  void reset()
  {
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["startTime"] << startTime;
    value["cycleTime"] << cycleTime;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["startTime"] >> startTime;
    value["cycleTime"] >> cycleTime;
  }
};
