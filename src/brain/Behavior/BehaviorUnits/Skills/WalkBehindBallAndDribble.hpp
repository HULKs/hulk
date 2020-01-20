#pragma once
#include "Behavior/Units.hpp"
#include "Tools/Math/Geometry.hpp"
#include "Tools/Math/Range.hpp"

/**
 * @brief Walk behind ball and dribble it
 * @param d dataset containing information about the current world state
 * @param target A walk target attached to the ball, usually a kick pose
 * @param velocity The velocity to be used when appraching the ball. Max. velocity is default.
 * @return A walk command to the ball target using the DRIBBLE walking mode
 */
ActionCommand walkBehindBallAndDribble(const DataSet& d, const Pose& target,
                                       const Velocity& velocity = Velocity())
{
  const Vector2f absOwnGoal = Vector2f(-d.fieldDimensions.fieldLength / 2.0f, 0.0f);
  const Vector2f absBall = d.teamBallModel.position;
  const Vector2f absRobot = d.robotPosition.pose.position;
  const Vector2f ownGoalToBall = absBall - absOwnGoal;
  const Vector2f ownGoalToRobot = absRobot - absOwnGoal;
  const Vector2f robotProjectedToLine =
      ownGoalToRobot.dot(ownGoalToBall) /
          (ownGoalToBall.squaredNorm() + std::numeric_limits<float>::epsilon()) * ownGoalToBall +
      absOwnGoal;

  // the minimum gap between the ball and the robot projected to the line from own goal to ball
  const float minGapToInterceptBall = 0.5f;
  if (ownGoalToBall.norm() - (robotProjectedToLine - absOwnGoal).norm() > minGapToInterceptBall)
  {
    // asymptotically approach line between own goal and ball until gap is closed
    const float interceptionFactor = 1.0f / 3.0f;
    const Vector2f projectionToBall = absBall - robotProjectedToLine;
    const Vector2f newTarget =
        d.robotPosition.fieldToRobot(robotProjectedToLine + interceptionFactor * projectionToBall);

    const Vector2f relBall = d.robotPosition.fieldToRobot(absBall);
    // add offset to distance to ball to make interpolation smoother
    const float distanceToBallOffset = 0.5f;
    // clip distance to ball to [0.0f, 1.0f]
    const float alpha = Range<float>::clipToZeroOne(relBall.norm() - distanceToBallOffset);
    // interpolate between newTarget and dribble target based on distance to ball
    const Vector2f interpolatedTarget = alpha * newTarget + (1.0f - alpha) * target.position;
    return ActionCommand::walk(Pose(interpolatedTarget, target.orientation), WalkMode::PATH,
                               velocity, InWalkKickType::NONE, KickFoot::NONE,
                               d.strikerAction.target);
  }
  else
  {
    return ActionCommand::walk(target, WalkMode::DRIBBLE, velocity, InWalkKickType::NONE,
                               KickFoot::NONE, d.strikerAction.target);
  }
}
