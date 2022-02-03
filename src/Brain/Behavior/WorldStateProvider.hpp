#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/WorldState.hpp"
#include "Framework/Module.hpp"


class Brain;

class WorldStateProvider : public Module<WorldStateProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"WorldStateProvider"};

  explicit WorldStateProvider(const ModuleManagerInterface& manager);

  void cycle() override;

private:
  bool checkBallInCorner(const Vector2f& absBallPosition);

  const Dependency<RobotPosition> robotPosition_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<FieldDimensions> fieldDimensions_;
  Production<WorldState> worldState_;

  /// whether the ball is free (i.e. the center circle may be entered)
  bool ballIsFree_;

  bool ballInOwnHalf_;
  bool ballInLeftHalf_;
  bool ballInCorner_;
  bool ballInPenaltyArea_;
  bool ballInGoalBoxArea_;
  bool ballIsToMyLeft_;
  bool ballInCenterCircle_;
  bool robotInOwnHalf_;
  bool robotInLeftHalf_;
  bool robotInPenaltyArea_;
  bool robotInGoalBoxArea_;

  const float hysteresis_ = 0.25f;

  const Parameter<float> ballInCornerThreshold_;
  const Parameter<float> ballInCornerXThreshold_;
  const Parameter<float> ballInCornerYThreshold_;
};
