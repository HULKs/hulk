#pragma once

#include "Data/BishopPosition.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/SupportingPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamPlayers.hpp"
#include "Data/WorldState.hpp"
#include "Framework/Module.hpp"


class Brain;

class BishopPositionProvider : public Module<BishopPositionProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name = "BishopPositionProvider";
  BishopPositionProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  /**
   * @brief whether the bishop should be aggressive, i. e. it should move close to the
   * opponent's goal
   * @return true if the bishop should be aggressive
   */
  bool beAggressive() const;

  Parameter<float> minimumAngle_;
  const Parameter<float> distanceToBall_;
  const Parameter<bool> allowAggressiveBishop_;

  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<PlayingRoles> playingRoles_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<SupportingPosition> supportingPosition_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<TeamPlayers> teamPlayers_;
  const Dependency<WorldState> worldState_;
  Production<BishopPosition> bishopPosition_;

  /// aggressiveBishopLineX is used to make sure that the bishop does not move too close to our own
  /// goal
  const float aggressiveBishopLineX_;
};
