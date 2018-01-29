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
  WorldStateProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  void updateBallThresholds(const bool ballInOwnHalf, const bool ballInLeftHalf);
  void updateRobotThresholds(const bool robotInOwnHalf, const bool robotInLeftHalf);

  const Dependency<RobotPosition> robotPosition_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<FieldDimensions> fieldDimensions_;
  Production<WorldState> worldState_;

  /// the current threshold for ball in own half decision
  float currentBallXThreshold_;
  /// the current threshold for ball in left half decision
  float currentBallYThreshold_;
  /// the current threshold for robot in own half decision
  float currentRobotXThreshold_;
  /// the current threshold for robot in left half decision
  float currentRobotYThreshold_;

  /// whether the ball is free (i.e. the center circle may be entered)
  bool ballIsFree_ = false;
};
