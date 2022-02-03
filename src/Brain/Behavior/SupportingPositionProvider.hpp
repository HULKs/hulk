#pragma once

#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/SupportingPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamPlayers.hpp"
#include "Data/WorldState.hpp"
#include "Framework/Module.hpp"


class Brain;

class SupportingPositionProvider : public Module<SupportingPositionProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"SupportingPositionProvider"};
  SupportingPositionProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<PlayingRoles> playingRoles_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<WorldState> worldState_;
  Production<SupportingPosition> supportingPosition_;

  Parameter<float> minimumAngle_;
  const Parameter<float> distanceToBall_;
  const Parameter<float> supporterClipGoalLineOffsetX_;
};
