#pragma once

#include <array>

#include "Data/ActionCommand.hpp"
#include "Framework/DataType.hpp"

class MotionActivation : public DataType<MotionActivation>
{
public:
  /// the name of this DataType
  DataTypeName name__{"MotionActivation"};
  /// the motion that the body should execute
  ActionCommand::Body::MotionType activeMotion{ActionCommand::Body::MotionType::DEAD};
  /// the amount of activeness that a motion should have
  ActionCommand::Body::MotionTypeArray<float> activations{};
  /// the amount of activeness that the headMotion should have
  float headMotionActivation{0.f};
  /// whether the head can be currently used independently
  bool headCanBeUsed{false};
  /// whether the arms can be currently used independently
  bool armsCanBeUsed{false};
  /**
   * @brief reset resets the activations
   */
  void reset() override
  {
    activeMotion = ActionCommand::Body::MotionType::DEAD;
    activations.fill(0.f);
    headMotionActivation = 0.f;
    headCanBeUsed = false;
    armsCanBeUsed = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["activeMotion"] << static_cast<int>(activeMotion);
    value["activations"] << activations;
    value["headMotionActivation"] << headMotionActivation;
    value["headCanBeUsed"] << headCanBeUsed;
    value["armsCanBeUsed"] << armsCanBeUsed;
  }

  void fromValue(const Uni::Value& value) override
  {
    unsigned int valueRead = 0;
    value["activeMotion"] >> valueRead;
    activeMotion = static_cast<ActionCommand::Body::MotionType>(valueRead);
    value["activations"] >> activations;
    value["headMotionActivation"] >> headMotionActivation;
    value["headCanBeUsed"] >> headCanBeUsed;
    value["armsCanBeUsed"] >> armsCanBeUsed;
  }
};
