#pragma once

#include <vector>

#include "Framework/DataType.hpp"
#include "Tools/Math/Circle.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Time.hpp"

class BallData : public DataType<BallData>
{
public:
  /// the name of this DataType
  DataTypeName name = "BallData";
  /// the positions of balls (where they touch the ground)
  VecVector2f positions;
  /// the image positions of balls
  std::vector<Circle<int>> imagePositions;
  /// filtered ball of last cycle projected onto the image
  Circle<int> filteredProjectedBall;
  /// the timestamp of the image in which it was seen
  TimePoint timestamp;
  /**
   * @brief reset sets the ball to a defined state
   */
  void reset()
  {
    positions.clear();
    imagePositions.clear();
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
