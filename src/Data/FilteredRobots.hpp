#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"
#include "Tools/StateEstimation/KalmanFilter.hpp"

#include "Tools/Math/Eigen.hpp"

class FilteredRobots : public DataType<FilteredRobots>
{
public:
  struct Robot : public Uni::To, public Uni::From
  {
    Robot() = default;

    Robot(Vector2f pos, Vector2f vel)
      : position(std::move(pos))
      , velocity(std::move(vel))
    {
    }
    /// the position of this robot
    Vector2f position{Vector2f::Zero()};
    /// the velocity of this robot
    Vector2f velocity{Vector2f::Zero()};

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["position"] << position;
      value["velocity"] << velocity;
    }

    void fromValue(const Uni::Value& value) override
    {
      value["position"] >> position;
      value["velocity"] >> velocity;
    }
  };
  /// the name of this DataType
  DataTypeName name__{"FilteredRobots"};
  std::vector<Robot> robots;
  bool valid{false};
  /**
   * @brief reset invalidates the data
   */
  void reset() override
  {
    robots.clear();
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["robots"] << robots;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["robots"] >> robots;
    value["valid"] >> valid;
  }
};
