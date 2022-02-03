#pragma once

#include "Framework/DataType.hpp"

class PenaltyKeeperAction : public DataType<PenaltyKeeperAction>
{
public:
  /// the name of this DataType
  DataTypeName name__{"PenaltyKeeperAction"};
  /**
   * @enum Type enumerates the possible types of action for a striker
   */
  enum Type
  {
    /// jump left
    JUMP_LEFT,
    /// jump right
    JUMP_RIGHT,
    /// squat (a leg spread- sit)
    SQUAT,
    /// wait for the striker to play.
    WAIT
  };
  /// true iff this struct is valid
  bool valid = false;
  /// the type of the action
  Type type;

  void reset() override
  {
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["valid"] << valid;
    value["type"] << static_cast<int>(type);
  }
  void fromValue(const Uni::Value& value) override
  {
    value["valid"] >> valid;
    int readNumber = 0;
    value["type"] >> readNumber;
    type = static_cast<Type>(readNumber);
  }
};
