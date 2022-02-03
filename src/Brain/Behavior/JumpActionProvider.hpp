#pragma once

#include "Brain/Behavior/BehaviorParameters.hpp"
#include "Data/BallState.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/JumpAction.hpp"
#include "Data/RobotPosition.hpp"
#include "Framework/Module.hpp"

class Brain;

/**
 * @brief JumpActionProvider
 */
class JumpActionProvider : public Module<JumpActionProvider, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"JumpActionProvider"};
  JumpActionProvider(const ModuleManagerInterface& manager);
  void cycle();

private:
  const Dependency<BallState> ballState_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<FieldDimensions> fieldDimensions_;

  const Parameter<float> standingRobotRadius_;
  const Parameter<float> squattedRobotRadius_;
  const Parameter<float> jumpedRobotRadius_;
  Production<JumpAction> jumpAction_;
};
