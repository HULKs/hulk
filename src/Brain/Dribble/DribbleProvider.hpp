#pragma once

#include "Data/ActionCommand.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/DribbleData.hpp"
#include "Data/PathPlannerData.hpp"
#include "Data/TeamBallModel.hpp"
#include "Framework/Module.hpp"

class Brain;

/**
 * @brief DribbleProvider provides necessary request and decisions while WalkMode::DRIBBLE
 */
class DribbleProvider : public Module<DribbleProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"DribbleProvider"};

  explicit DribbleProvider(const ModuleManagerInterface& manager);

  void cycle() override;

private:
  const Dependency<ActionCommand> actionCommand_;
  const Dependency<TeamBallModel> teamBallModel_;
  const Dependency<PathPlannerData> pathPlannerData_;

  /// start dribbling when the alignment difference is smaller than this threshold
  Parameter<float> dribbleAngleTolerance_;
  /// the speed stepping forwards while dribbling [m/step]
  const Parameter<float> dribbleSpeed_;
  /// the maximum distance to the target line the robot is considered safe to dribble
  const Parameter<float> maxDistanceToDribbleLine_;
  /// the maximum distance to the dribble target the robot is considered safe to dribble
  const Parameter<float> maxDistanceToDribblePosition_;

  Production<DribbleData> dribbleData_;

  /**
   * @brief determines whether the dribble target is reached and the robot can start to dribble
   * @return whether dribble target is reached
   */
  bool isDribbleTargetReached();

  /// whether the dribble target was reached last time, used for hysteresis
  bool wasDribbleTargetReachedLastCycle_{false};
};
