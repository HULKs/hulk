#pragma once

#include "Data/BallState.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/PenaltyKeeperAction.hpp"
#include "Framework/Module.hpp"

class Brain;

class PenaltyKeeperActionProvider : public Module<PenaltyKeeperActionProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"PenaltyKeeperActionProvider"};
  /**
   * @brief PenaltyKeeperActionProvider initializes members
   * @param manager a reference to brain
   */
  PenaltyKeeperActionProvider(const ModuleManagerInterface& manager);
  /**
   * @brief cycle updates the striker action
   */
  void cycle();

private:
  /// Tolerance value for goalLineHalfWithTolerance_; default +- 0.2m
  const Parameter<float> goalLineLenTolerance_;
  /// Y-axis distance of ball to robot to determine jump or squat.; default 0.2m
  const Parameter<float> squatThreshold_;
  /// Default 25cm bias to be safe. destVector is lengthened by this; default 0.25m
  const Parameter<float> ballDestinationTolerance_;
  /// Minimum x distance from ball dest to robot to calculate squat.; default 0.05m
  const Parameter<float> minBallDestinationToRobotThresh_;

  const Dependency<FieldDimensions> fieldDimensions_;

  const Dependency<BallState> ballState_;

  const Dependency<GameControllerState> gameControllerState_;

  Production<PenaltyKeeperAction> penaltyAction_;

  /// store the state; default = WAIT
  PenaltyKeeperAction::Type previousActionType_;
  /// Half of goal line + tolerance value.
  const float goalLineHalfWithTolerance_;
};
