#pragma once

#include "Framework/DataType.hpp"

#include "Tools/Math/Circle.hpp"
#include "Tools/Time.hpp"

class CircleData : public DataType<CircleData> {
public:
  /// the name of this DataType
  DataTypeName name = "CircleData";
  /// position and radius of the detected circle
  Circle<float> circle;
  /// the timestamp of the image in which it was seen
  TimePoint timestamp;
  /// whether the circle has been seen
  bool found;
  /**
   * @brief reset sets the circle to a defined state
   */
  void reset()
  {
    found = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["circle"] << circle;
    value["timestamp"] << timestamp;
    value["found"] << found;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["circle"] >> circle;
    value["timestamp"] >> timestamp;
    value["found"] >> found;
  }
};
