#pragma once
#include "Behavior/Units.hpp"

/**
 * @brief Walk behind ball and dribble it
 * @param d dataset containing information about the current world state
 * @param target A walk target attached to the ball, usually a kick pose
 * @param velocity The velocity to be used when appraching the ball. Max. velocity is default.
 * @return A walk command to the ball target using the DRIBBLE walking mode
 */
ActionCommand walkBehindBallAndDribble(const DataSet& d, const Pose& target, const Velocity& velocity = Velocity())
{
  return ActionCommand::walk(target, WalkMode::DRIBBLE, velocity);
}
