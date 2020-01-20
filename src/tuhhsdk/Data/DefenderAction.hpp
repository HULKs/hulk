#pragma once

#include "Framework/DataType.hpp"
#include "Tools/BallUtils.hpp"
#include "Tools/Math/Pose.hpp"
#include "Tools/Math/Eigen.hpp"


class DefenderAction : public DataType<DefenderAction>
{
public:
  /// the name of this DataType
  DataTypeName name = "DefenderAction";
  /**
   * @enum Type enumerates the possible types of action for a defender
   */
  enum Type
  {
    DEFEND,
    GENUFLECT
  };

  /// true iff this struct is valid
  bool valid = false;
  /// the type of the action
  Type type = Type::DEFEND;

  /**
   * @brief reset does nothing
   */
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
