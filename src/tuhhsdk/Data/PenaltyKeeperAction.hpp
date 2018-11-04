#pragma once

#include "Framework/DataType.hpp"

class PenaltyKeeperAction : public DataType<PenaltyKeeperAction>
{
public:
  /// the name of this DataType
  DataTypeName name = "PenaltyKeeperAction";
  /**
   * @enum Type enumerates the possible types of action for a striker
   */
  enum Type
  {
    /// jump left
    JUMP_LEFT,
    /// jump right
    JUMP_RIGHT,
    /// genuflect (a leg spread- sit)
    GENUFLECT,
    /// wait for the striker to play.
    WAIT
  };
  /// true iff this struct is valid
  bool valid = false;
  /// the type of the action
  Type type;
  /**
   * @brief reset does nothing
   */
  void reset()
  {
    valid = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["valid"] << valid;
    value["type"] << static_cast<int>(type);
  }
  virtual void fromValue(const Uni::Value& value)
  {
    value["valid"] >> valid;
    int readNumber = 0;
    value["type"] >> readNumber;
    type = static_cast<Type>(readNumber);
  }
};
