#pragma once

#include "Data/BallState.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/PenaltyStrikerAction.hpp"
#include "Data/RobotPosition.hpp"
#include "Framework/Module.hpp"

class Brain;

class PenaltyStrikerActionProvider : public Module<PenaltyStrikerActionProvider, Brain>
{
public:
  ModuleName name = "PenaltyStrikerActionProvider";

  PenaltyStrikerActionProvider(const ModuleManagerInterface& manager);

  void cycle();

private:
  const Dependency<BallState> ballState_;
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<RobotPosition> robotPosition_;
  const Parameter<float> aimAtCornerFactor_;
  const Parameter<int> useOnlyThisFoot_;
  const Parameter<Vector2f> distanceToBallKick_;
  int lastSign_;
  float penaltyTargetOffset_;
  Production<PenaltyStrikerAction> penaltyStrikerAction_;
};
