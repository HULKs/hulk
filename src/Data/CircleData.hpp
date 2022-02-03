#pragma once

#include "Framework/DataType.hpp"

#include "Hardware/Clock.hpp"
#include "Tools/Math/Circle.hpp"

class CircleData : public DataType<CircleData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"CircleData"};
  /// position and radius of the detected circle
  Circle<float> circle;
  /// the timestamp of the image in which it was seen
  Clock::time_point timestamp;
  /// whether the circle has been seen
  bool found;
  /**
   * @brief reset sets the circle to a defined state
   */
  void reset() override
  {
    found = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["circle"] << circle;
    value["timestamp"] << timestamp;
    value["found"] << found;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["circle"] >> circle;
    value["timestamp"] >> timestamp;
    value["found"] >> found;
  }
};
