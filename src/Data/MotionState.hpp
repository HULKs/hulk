#pragma once

#include "Data/ActionCommand.hpp"
#include "Framework/DataType.hpp"

class MotionState : public DataType<MotionState>
{
public:
  /// the name of this DataType
  DataTypeName name__{"MotionState"};
  /// the motion that the body (legs + potentially arms + potentially head) executes
  ActionCommand::Body::MotionType bodyMotion = ActionCommand::Body::MotionType::DEAD;
  /// the motion that the left arm executes
  ActionCommand::Arm::MotionType leftArmMotion = ActionCommand::Arm::MotionType::BODY;
  /// the motion that the right arm executes
  ActionCommand::Arm::MotionType rightArmMotion = ActionCommand::Arm::MotionType::BODY;
  /// the motion that the head executes
  ActionCommand::Head::MotionType headMotion = ActionCommand::Head::MotionType::BODY;
  /// the angles (this is the motion output that is send to the robot interface)
  JointsArray<float> angles{};
  /// the stiffnesses
  JointsArray<float> stiffnesses{};

  /**
   * @brief reset sets the robot dead
   */
  void reset() override
  {
    bodyMotion = ActionCommand::Body::MotionType::DEAD;
    leftArmMotion = ActionCommand::Arm::MotionType::BODY;
    rightArmMotion = ActionCommand::Arm::MotionType::BODY;
    headMotion = ActionCommand::Head::MotionType::BODY;
    angles.fill(0);
    stiffnesses.fill(0);
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["bodyMotion"] << static_cast<int>(bodyMotion);
    value["leftArmMotion"] << static_cast<int>(leftArmMotion);
    value["rightArmMotion"] << static_cast<int>(rightArmMotion);
    value["headMotion"] << static_cast<int>(headMotion);
    value["angles"] << angles;
    value["stiffnesses"] << stiffnesses;
  }

  void fromValue(const Uni::Value& value) override
  {
    int readValue = 0;
    value["bodyMotion"] >> readValue;
    bodyMotion = static_cast<ActionCommand::Body::MotionType>(readValue);
    value["leftArmMotion"] >> readValue;
    leftArmMotion = static_cast<ActionCommand::Arm::MotionType>(readValue);
    value["rightArmMotion"] >> readValue;
    rightArmMotion = static_cast<ActionCommand::Arm::MotionType>(readValue);
    value["headMotion"] >> readValue;
    headMotion = static_cast<ActionCommand::Head::MotionType>(readValue);
    value["angles"] >> angles;
    value["stiffnesses"] >> stiffnesses;
  }
};
