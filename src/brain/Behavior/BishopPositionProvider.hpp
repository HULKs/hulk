#pragma once

#include "Data/BishopPosition.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/WorldState.hpp"
#include "Framework/Module.hpp"


class Brain;

class BishopPositionProvider : public Module<BishopPositionProvider, Brain>
{
public:
  BishopPositionProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<PlayingRoles> playingRoles_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<WorldState> worldState_;
  Production<BishopPosition> bishopPosition_;
};
