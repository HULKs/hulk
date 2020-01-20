#pragma once

#include "Tools/Time.hpp"
#include "Framework/DataType.hpp"


class WhistleData : public DataType<WhistleData> {
public:
  /// the name of this DataType
  DataTypeName name = "WhistleData";
  /// the last timestamp when a whistle has been detected
  TimePoint lastTimeWhistleHeard;
  /**
   * @brief reset does nothing
   */
  void reset() override
  {
  }

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
