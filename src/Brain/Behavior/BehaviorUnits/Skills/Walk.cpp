#include "Brain/Behavior/Units.hpp"
#include "Tools/Math/Range.hpp"

ActionCommand walkBehindBall(const DataSet& /*d*/, const Pose& target, const Velocity& velocity)
{
  return ActionCommand::walk(target, ActionCommand::Body::WalkMode::WALK_BEHIND_BALL, velocity);
}

ActionCommand walkBehindBallAndDribble(const DataSet& d, const Pose& walkTarget,
                                       const Vector2f& ballTarget, const Velocity& velocity)
{
  const Vector2f absOwnGoal = Vector2f(-d.fieldDimensions.fieldLength / 2.0f, 0.0f);
  const Vector2f& absRobot = d.robotPosition.pose.position();
  const Vector2f ownGoalToBall = d.teamBallModel.absPosition - absOwnGoal;
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
    const Vector2f projectionToBall = d.teamBallModel.absPosition - robotProjectedToLine;
    const Vector2f newTarget =
        d.robotPosition.fieldToRobot(robotProjectedToLine + interceptionFactor * projectionToBall);

    const Vector2f relBall = d.robotPosition.fieldToRobot(d.teamBallModel.absPosition);
    // add offset to distance to ball to make interpolation smoother
    const float distanceToBallOffset = 0.5f;
    // clip distance to ball to [0.0f, 1.0f]
    const float alpha = Range<float>::clipToZeroOne(relBall.norm() - distanceToBallOffset);
    // interpolate between newTarget and dribble walkTarget based on distance to ball
    const Vector2f interpolatedTarget = alpha * newTarget + (1.0f - alpha) * walkTarget.position();
    return ActionCommand::walk(Pose(interpolatedTarget, walkTarget.angle()),
                               ActionCommand::Body::WalkMode::PATH, velocity, InWalkKickType::NONE,
                               KickFoot::NONE, ballTarget)
        .combineHead(activeVision(d, VisionMode::BALL_TRACKER));
  }
  return ActionCommand::walk(walkTarget, ActionCommand::Body::WalkMode::DRIBBLE, velocity,
                             InWalkKickType::NONE, KickFoot::NONE, ballTarget)
      .combineHead(activeVision(d, VisionMode::BALL_TRACKER));
}

ActionCommand walkToPose(const DataSet& d, const Pose& pose, bool absolute,
                         const ActionCommand::Body::WalkMode walkMode, const Velocity& velocity,
                         float hysteresis, const ActionCommand& fallback)
{
  // Velocity mode doesn't make sense for this action
  assert(walkMode != ActionCommand::Body::WalkMode::VELOCITY);

  Pose absTarget = absolute ? pose : d.robotPosition.robotToField(pose);
  const float maxDistanceToBorder = d.fieldDimensions.fieldBorderStripWidth / 2;

  if (std::abs(absTarget.x()) > (d.fieldDimensions.fieldLength / 2 + maxDistanceToBorder))
  {
    const float signX = absTarget.x() < 0.0f ? -1.0f : 1.0f;
    absTarget.x() = signX * (d.fieldDimensions.fieldLength / 2 + maxDistanceToBorder);
    absTarget.angle() = std::atan2(absTarget.y(), absTarget.x());
  }

  if (std::abs(absTarget.y()) > (d.fieldDimensions.fieldWidth / 2 + maxDistanceToBorder))
  {
    const float signY = absTarget.y() < 0.0f ? -1.0f : 1.0f;
    absTarget.y() = signY * (d.fieldDimensions.fieldWidth / 2 + maxDistanceToBorder);
    absTarget.angle() = std::atan2(absTarget.y(), absTarget.x());
  }

  const Pose relTarget = d.robotPosition.fieldToRobot(absTarget);

  const bool near =
      relTarget.position().squaredNorm() < 0.01f && std::abs(relTarget.angle()) < 3 * TO_RAD;
  const bool near2 = relTarget.position().squaredNorm() < 0.01f * hysteresis * hysteresis &&
                     std::abs(relTarget.angle()) < 3 * hysteresis * TO_RAD;

  if (near || (d.lastRequestedBodyMotionType == ActionCommand::Body::MotionType::STAND && near2))
  {
    return fallback;
  }

  return ActionCommand::walk(relTarget, walkMode, velocity);
}
