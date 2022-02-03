#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Pose.hpp"

class StepPlan : public DataType<StepPlan>
{
public:
  /// the name of this DataType
  DataTypeName name__{"StepPlan"};

  /// Forward step size [m/step] Forward is positive.
  float forward{0.f};
  /// Sideways step size [m/step] Left is positive.
  float left{0.f};
  /// Turn size in [rad/step] Anti-clockwise is positive.
  float turn{0.f};
  /// the maximum step sizes configured [[m,m], rad]
  Pose maxStepSize;
  /// whether this data is valid
  bool valid = false;

  void reset() override
  {
    forward = 0.f;
    left = 0.f;
    turn = 0.f;
    maxStepSize = Pose{};
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["forward"] << forward;
    value["left"] << left;
    value["turn"] << turn;
    value["maxStepSize"] << maxStepSize;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["forward"] >> forward;
    value["left"] >> left;
    value["turn"] >> turn;
    value["maxStepSize"] >> maxStepSize;
    value["valid"] >> valid;
  }
};
