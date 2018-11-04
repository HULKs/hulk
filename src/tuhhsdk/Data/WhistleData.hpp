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
  void reset()
  {
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["lastTimeWhistleHeard"] << lastTimeWhistleHeard;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["lastTimeWhistleHeard"] >> lastTimeWhistleHeard;
  }
};
