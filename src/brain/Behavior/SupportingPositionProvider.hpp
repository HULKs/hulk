#pragma once

#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/SupportingPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamPlayers.hpp"
#include "Framework/Module.hpp"


class Brain;

class SupportingPositionProvider : public Module<SupportingPositionProvider, Brain>
{
public:
  SupportingPositionProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  void findPassTarget(const TeamPlayer*& passTarget);

  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<PlayerConfiguration> playerConfiguration_;
  const Dependency<PlayingRoles> playingRoles_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<TeamPlayers> teamPlayers_;
  Production<SupportingPosition> supportingPosition_;

  const Parameter<float> minimumDistance_;
  bool wasObstructing_;
};
