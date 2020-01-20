#pragma once

#include <array>

#include "Data/MotionRequest.hpp"
#include "Framework/DataType.hpp"

class MotionActivation : public DataType<MotionActivation>
{
public:
  /// the name of this DataType
  DataTypeName name = "MotionActivation";
  /// the motion that the body should execute
  MotionRequest::BodyMotion activeMotion;
  /// the amount of activeness that a motion should have
  std::array<float, static_cast<unsigned int>(MotionRequest::BodyMotion::NUM)> activations;
  /// the amount of activeness that the headMotion should have
  float headMotionActivation;
  /// whether the head can be currently used independently
  bool headCanBeUsed;
  /// whether the arms can be currently used independently
  bool armsCanBeUsed;
  /// whether an inteprolation started this cycle
  bool startInterpolation;
  /**
   * @brief reset resets the activations
   */
  void reset() override
  {
    activations.fill(0.f);
    headMotionActivation = 0.f;
    headCanBeUsed = false;
    armsCanBeUsed = false;
    startInterpolation = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["activeMotion"] << static_cast<int>(activeMotion);
    value["activations"] << activations;
    value["headMotionActivation"] << headMotionActivation;
    value["headCanBeUsed"] << headCanBeUsed;
    value["armsCanBeUsed"] << armsCanBeUsed;
    value["startInterpolation"] << startInterpolation;
  }

  void fromValue(const Uni::Value& value) override
  {
    int valueRead = 0;
    value["activeMotion"] >> valueRead;
    activeMotion = static_cast<MotionRequest::BodyMotion>(valueRead);
    value["activations"] >> activations;
    value["headMotionActivation"] >> headMotionActivation;
    value["headCanBeUsed"] >> headCanBeUsed;
    value["armsCanBeUsed"] >> armsCanBeUsed;
    value["startInterpolation"] >> startInterpolation;
  }
};
