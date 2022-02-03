#pragma once

#include "Data/LoserPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Framework/Module.hpp"

class Brain;

class LoserPositionProvider : public Module<LoserPositionProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"LoserPositionProvider"};

  /**
   * @brief LoserPositionProvider The constructor
   * @param manager Reference to the ModuleManagerInterface (e.g. brain)
   */
  explicit LoserPositionProvider(const ModuleManagerInterface& manager);

  void cycle() override;

private:
  Dependency<TeamBallModel> teamBallModel_;

  /// The absolute position where the loser should go
  Production<LoserPosition> loserPosition_;

  /// the last known location of the team ball in absolute coordinates
  Vector2f lastKnownTeamBallPosition_{Vector2f::Zero()};
};
