#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/KinematicMatrix.hpp"

class WalkManagerOutput : public DataType<WalkManagerOutput>
{
public:
  /// the name of this DataType
  DataTypeName name__{"WalkManagerOutput"};

  enum class RequestAction
  {
    WALK,
    STAND,
    RESET,
  };

  /// whether the walking is active
  bool isActive;
  /// the action to request from the walking engine
  RequestAction action;
  /// Forward step size [m/step] Forward is positive.
  float forward{0.f};
  /// Sideways step size [m/step] Left is positive.
  float left{0.f};
  /// Turn size in [rad/step] Anti-clockwise is positive.
  float turn{0.f};
  /// whether data of this DataType is valid
  bool valid = false;

  /// the offset to apply to the swing foot while executing a step (used for in-walk kick)
  std::function<KinematicMatrix(const float phase)> getKickFootOffset =
      std::function<KinematicMatrix(float)>();

  void reset() override
  {
    isActive = false;
    action = RequestAction::STAND;
    forward = 0.f;
    left = 0.f;
    turn = 0.f;
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["active"] << isActive;
    value["action"] << static_cast<unsigned int>(action);
    value["forward"] << forward;
    value["left"] << left;
    value["turn"] << turn;
    value["valid"] << valid;
  }
  void fromValue(const Uni::Value& value) override
  {
    value["active"] >> isActive;
    value["forward"] >> forward;
    value["left"] >> left;
    value["turn"] >> turn;
    value["valid"] >> valid;
  }
};
