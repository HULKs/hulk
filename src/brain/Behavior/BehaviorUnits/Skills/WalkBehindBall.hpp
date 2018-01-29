#pragma once
#include "Behavior/Units.hpp"

/**
 * @brief Walk around the ball properly and ensure correct orientation before approaching the ball
 * @param d dataset containing information about the currentworld state
 * @param target A walk target attached to the ball, usually a kick pose
 * @param velocity The velocity to be used when appraching the ball. Max. velocity is default.
 * @return A walk command to the ball target using the WALK_BEHIND_BALL walking mode
 */
ActionCommand walkBehindBall(const DataSet& d, const Pose& target, const Velocity& velocity = Velocity())
{
  return ActionCommand::walk(target, WalkMode::WALK_BEHIND_BALL, velocity);
}
