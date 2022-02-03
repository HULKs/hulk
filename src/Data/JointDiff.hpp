#pragma once

#include "Data/MotionOutput.hpp"
#include "Framework/DataType.hpp"

class JointDiff : public DataType<JointDiff, MotionOutput>
{
public:
  DataTypeName name__{"JointDiff"};

  JointsArray<float> angles{};
  float leftArmSum = 0.0f;
  float leftLegSum = 0.0f;
  float rightArmSum = 0.0f;
  float rightLegSum = 0.0f;
  /// The body sum includes all joints of the body. It excludes head pitch and yaw.
  float bodySum = 0.0f;
  /// The head sum combines head pitch and yaw.
  float headSum = 0.0f;
  bool valid = false;

  void reset() override
  {
    angles.fill(0.0f);
    leftArmSum = 0.0f;
    leftLegSum = 0.0f;
    rightArmSum = 0.0f;
    rightLegSum = 0.0f;
    bodySum = 0.0f;
    headSum = 0.0f;
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["angles"] << angles;
    value["leftArmSum"] << leftArmSum;
    value["leftLegSum"] << leftLegSum;
    value["rightArmSum"] << rightArmSum;
    value["rightLegSum"] << rightLegSum;
    value["bodySum"] << bodySum;
    value["headSum"] << headSum;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["angles"] >> angles;
    value["leftArmSum"] >> leftArmSum;
    value["leftLegSum"] >> leftLegSum;
    value["rightArmSum"] >> rightArmSum;
    value["rightLegSum"] >> rightLegSum;
    value["bodySum"] >> bodySum;
    value["headSum"] >> headSum;
    value["valid"] >> valid;
  }
};
