#pragma once

#include "Data/GameControllerState.hpp"
#include "Data/KeeperAction.hpp"
#include "Data/ReplacementKeeperAction.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/WorldState.hpp"
#include "Framework/Module.hpp"

class Brain;

class ReplacementKeeperActionProvider : public Module<ReplacementKeeperActionProvider, Brain>
{
public:
  ModuleName name__{"ReplacementKeeperActionProvider"};
  ReplacementKeeperActionProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  void considerSetPlay();

  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<KeeperAction> keeperAction_;
  const Dependency<WorldState> worldState_;
  Production<ReplacementKeeperAction> replacementKeeperAction_;
};
