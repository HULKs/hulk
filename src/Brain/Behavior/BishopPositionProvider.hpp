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
  ModuleName name__{"BishopPositionProvider"};
  BishopPositionProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  enum class Side
  {
    LEFT = 1,
    RIGHT = -1
  };

  Parameter<float> minimumAngle_;
  const Parameter<float> distanceToBall_;
  const Parameter<bool> allowAggressiveBishop_;
  /// the default position is used to be a pass target when the striker clears the ball
  const Parameter<Vector2f> defaultPositionOffset_;
  /// the corner kick position is in front of the opponent's goal to score after the set play
  /// striker completes a corner kick
  const Parameter<Vector2f> cornerKickOffset_;
  /// the goalhanger poistion is in front and to the side of the opponent's goal to finish after an
  /// attempt by the striker to score a goal
  const Parameter<Vector2f> goalhangerOffset_;

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

  /// the side the bishop should be on
  Side side_{Side::LEFT};

  /**
   * @brief determine which side (left/right) the bishop should be on
   *
   * This function updates side_, but only if the ball is on our own half to not obstruct the
   * striker. The bishop should generally be on the side the ball is not on.
   */
  void determineLeftOrRight();
};
