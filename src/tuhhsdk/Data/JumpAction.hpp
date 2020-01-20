#pragma once

#include "Framework/DataType.hpp"


class JumpAction : public DataType<JumpAction>
{
public:
  /// the name of this DataType
  DataTypeName name = "JumpAction";

  /**
   * @enum Type enumerates the possible types of action for a keeper
   * all values must be powers of two for the permission management to work
   */
  enum class Type
  {
    NONE,
    SQUAT,
    JUMP_LEFT,
    JUMP_RIGHT
  };

  /// wheter the robot could stop the moving ball with a squat motion
  bool canCatchWithSquat = false;
  /// wheter the robot could stop the moving ball with a jump motion
  bool canCatchWithJump = false;

  /// whether the jump action is valid
  bool valid = false;

  /// the best jump type
  Type suggestedType = Type::NONE;

  /**
   * @brief invalidates the action
   */
  void reset()
  {
    canCatchWithSquat = false;
    canCatchWithJump = false;
    valid = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["valid"] << valid;
    value["suggestedType"] << static_cast<int>(suggestedType);
  }

  virtual void fromValue(const Uni::Value& value)
  {
    int readNumber;
    value["valid"] >> valid;
    value["suggestedType"] >> readNumber;
    suggestedType = static_cast<Type>(readNumber);
  }
};
