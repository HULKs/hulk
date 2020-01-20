#pragma once

#include "Behavior/Units.hpp"
#include "Modules/NaoProvider.h"
#include "WalkBehindBallAndDribble.hpp"


/**
 * @brief dribble creates an action command for dribbling the ball somewhere
 * @pre The team ball has to be seen.
 * @param d a dataset
 * @param kickPose the relative (!!!) kick pose
 * @return an action command for dribbling the ball somewhere
 */
ActionCommand dribble(const DataSet& d, const Pose kickPose)
{
  return walkBehindBallAndDribble(d, kickPose).combineHead(activeVision(d, VisionMode::BALL_TRACKER));
}
