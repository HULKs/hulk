#pragma once

#include <vector>

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Time.hpp"

class RobotData : public DataType<RobotData>
{
public:
  /// the name of this DataType
  DataTypeName name = "RobotData";
  /// a vector of detected robots in robot coordinates
  VecVector2f positions;
  /// the timestamp of the image these detections were derived from
  TimePoint timestamp;
  /**
   * @brief reset resets this datatype by clearing the position vector
   */
  void reset()
  {
    positions.clear();
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["positions"] << positions;
    value["timestamp"] << timestamp;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["positions"] >> positions;
    value["timestamp"] >> timestamp;
  }
};
