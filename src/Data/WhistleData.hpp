#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"


class WhistleData : public DataType<WhistleData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"WhistleData"};
  /// the last timestamp when a whistle has been detected
  Clock::time_point lastTimeWhistleHeard;
  /**
   * @brief reset does nothing
   */
  void reset() override {}

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["lastTimeWhistleHeard"] << lastTimeWhistleHeard;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["lastTimeWhistleHeard"] >> lastTimeWhistleHeard;
  }
};
