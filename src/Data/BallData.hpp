#pragma once

#include <vector>

#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"
#include "Tools/Math/Circle.hpp"
#include "Tools/Math/Eigen.hpp"

class BallData : public DataType<BallData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"BallData"};
  /// the positions of balls (where they touch the ground)
  VecVector2f positions;
  /// the image positions of balls
  std::vector<Circle<int>> imagePositions;
  /// the timestamp of the image in which it was seen
  Clock::time_point timestamp;

  bool valid = false;

  /**
   * @brief reset sets the ball to a defined state
   */
  void reset() override
  {
    valid = false;
    positions.clear();
    imagePositions.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["positions"] << positions;
    value["timestamp"] << timestamp;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["positions"] >> positions;
    value["timestamp"] >> timestamp;
  }
};
