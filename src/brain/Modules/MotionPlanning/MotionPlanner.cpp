#include <algorithm>
#include <cmath>
#include <limits>
#include <stdexcept>

#include "Data/MotionRequest.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Geometry.hpp"
#include "print.h"

#include "MotionPlanner.hpp"

MotionPlanner::MotionPlanner(const ModuleManagerInterface& manager)
  : Module(manager)
  , hybridAlignDistance_(*this, "hybridAlignDistance", [] {})
  , dribbleAlignDistance_(*this, "dribbleAlignDistance", [] {})
  , targetAlignDistance_(*this, "targetAlignDistance", [] {})
  , ballOffsetShiftAngle_(*this, "ballOffsetShiftAngle",
                          [this] { ballOffsetShiftAngle_() *= TO_RAD; })
  , ballOffsetDistance_(*this, "ballOffsetDistance", [] {})
  , ballOffsetTargetOrientationTolerance_(
        *this, "ballOffsetTargetOrientationTolerance",
        [this] { ballOffsetTargetOrientationTolerance_() *= TO_RAD; })
  , ballWeight_(*this, "ballWeight", [] {})
  , freeKickAreaWeight_(*this, "freeKickAreaWeight", [] {})
  , robotWeight_(*this, "robotWeight", [] {})
  , fallenRobotWeight_(*this, "fallenRobotWeight", [] {})
  , unknownObstacleWeight_(*this, "unknownObstacleWeight", [] {})
  , totalObstacleWeight_(*this, "totalObstacleWeight", [] {})
  , obstacleDisplacementAngle_(*this, "obstacleDisplacementAngle",
                               [this] { obstacleDisplacementAngle_() *= TO_RAD; })
  , strikerUsesOnlyLocalObstacles_(*this, "strikerUsesOnlyLocalObstacles", [] {})
  , ignoreGoalPostObstacles_(*this, "ignoreGoalPostObstacles", [] {})
  , enableCarefulDribbling_(*this, "enableCarefulDribbling", [] {})
  , carefulDribbleSpeed_(*this, "carefulDribbleSpeed", [] {})
  , carefulDribbleDistanceThreshold_(*this, "carefulDribbleDistanceThreshold", [] {})
  , footOffset_(*this, "footOffset", [] {})
  , groundLevelAvoidanceDistance_(*this, "groundLevelAvoidanceDistance", [] {})
  , shoulderLevelAvoidanceDistance_(*this, "shoulderLevelAvoidanceDistance", [] {})
  , dribblingAngleTolerance_(*this, "dribblingAngleTolerance",
                             [this] { dribblingAngleTolerance_() *= TO_RAD; })
  , slowBallApproachFactor_(*this, "slowBallApproachFactor", [] {})
  , maxDistToBallTargetLine_(*this, "maxDistToBallTargetLine", [] {})
  , walkAroundBallDistanceThreshold_(*this, "walkAroundBallDistanceThreshold", [] {})
  , walkAroudBallAngleThreshold_(*this, "walkAroudBallAngleThreshold",
                                 [this] { walkAroudBallAngleThreshold_() *= TO_RAD; })
  , motionRequest_(*this)
  , obstacleData_(*this)
  , teamObstacleData_(*this)
  , robotPosition_(*this)
  , ballState_(*this)
  , walkingEngineWalkOutput_(*this)
  , playingRoles_(*this)
  , motionPlannerOutput_(*this)
  , obstacleWeights_()
  , offsetBallTargetReached_(false)
  , walkAroundBallTargetReached_(true)
  , ignoreBallObstacle_(false)
  , ignoreRobotObstacles_(false)
  , lastfootdecision_(FootDecision::NONE)
{
  ballOffsetShiftAngle_() *= TO_RAD;
  obstacleDisplacementAngle_() *= TO_RAD;
  ballOffsetTargetOrientationTolerance_() *= TO_RAD;
  dribblingAngleTolerance_() *= TO_RAD;
  walkAroudBallAngleThreshold_ () *= TO_RAD;

  // Initialize obstacle-weight association
  obstacleWeights_.fill(unknownObstacleWeight_()); // Defaults to unknown obstacle weight
  obstacleWeights_[static_cast<int>(ObstacleType::BALL)] = ballWeight_();
  obstacleWeights_[static_cast<int>(ObstacleType::FREE_KICK_AREA)] = freeKickAreaWeight_();
  obstacleWeights_[static_cast<int>(ObstacleType::ANONYMOUS_ROBOT)] = robotWeight_();
  obstacleWeights_[static_cast<int>(ObstacleType::HOSTILE_ROBOT)] = robotWeight_();
  obstacleWeights_[static_cast<int>(ObstacleType::TEAM_ROBOT)] = robotWeight_();
  obstacleWeights_[static_cast<int>(ObstacleType::FALLEN_ANONYMOUS_ROBOT)] = fallenRobotWeight_();
  obstacleWeights_[static_cast<int>(ObstacleType::FALLEN_HOSTILE_ROBOT)] = fallenRobotWeight_();
  obstacleWeights_[static_cast<int>(ObstacleType::FALLEN_TEAM_ROBOT)] = fallenRobotWeight_();

  if (!totalObstacleWeight_())
  {
    print("MotionPlanner obstacle weight was initialized to 0, all obstacles will be ignored.",
          LogLevel::WARNING);
  }
}

void MotionPlanner::cycle()
{
  // Copy current MotionRequest to MotionPlannerOutput, this way the motionPlanner may modify the
  // request to pass on its results in form of its own output, without modifying the original
  // request. '&*' is needed because the dependency is not a real pointer.
  motionRequest_->copy(&*motionPlannerOutput_);
  // Only perform motionplanning when the robot is walking
  if (motionRequest_->bodyMotion != MotionRequest::BodyMotion::WALK)
  {
    return;
  }
  // When not in velocity mode, calculate the desired rotation and translation
  if (motionPlannerOutput_->walkData.mode != WalkMode::VELOCITY)
  {
    // Create an offset walk target if needed
    if (motionPlannerOutput_->walkData.mode == WalkMode::WALK_BEHIND_BALL)
    {
      // Sets a new offset target away from the ball
      setWalkBehindBallPosition(std::abs(ballOffsetShiftAngle_()));
    }
    else if (motionPlannerOutput_->walkData.mode == WalkMode::DRIBBLE)
    {
      // A small angle ensures a good transtion between walking around the ball and dribbling
      const float dribblingBallOffsetAngle = 2 * TO_RAD;
      setWalkBehindBallPosition(dribblingBallOffsetAngle);
    }
    else
    {
      // Reset these if not calling setWalkBehindBallPosition to prevent non-striker robots from
      // ignoring ball or robot obstacles or having weird walk targets.
      ignoreBallObstacle_ = false;
      ignoreRobotObstacles_ = false;
      offsetBallTargetReached_ = false;
      walkAroundBallTargetReached_ = false;
    }
    // Calculate the orientation the robot shoulder achieve.
    motionPlannerOutput_->walkData.target.orientation = calculateRotation();
    // The length of this vector represents the max. velocity limit, not a distance!
    motionPlannerOutput_->walkData.velocity.translation = calculateTranslation();
    // In DRIBBLE mode, change the walking mode to velocity after reaching the offset target
    // waypoint, to avoid braking when getting near the ball
    if (motionPlannerOutput_->walkData.mode == WalkMode::DRIBBLE && offsetBallTargetReached_)
    {
      // Copy the target orientation to the velocity because it is needed in velocity mode
      motionPlannerOutput_->walkData.velocity.rotation =
          motionPlannerOutput_->walkData.target.orientation;
      if (enableCarefulDribbling_() &&
          ballState_->position.norm() < carefulDribbleDistanceThreshold_())
      {
        // clip the dribbling velocity since the maximum walking speed might be quite fast
        motionPlannerOutput_->walkData.velocity =
            getClippedDribbleVelocity(motionPlannerOutput_->walkData.velocity);
        assert(!motionPlannerOutput_->walkData.velocity.isPercentage());
      }
      motionPlannerOutput_->walkData.mode = WalkMode::VELOCITY;
    }
  }
  // Serialize and send debug data
  debug().update(mount_, *this);
}

Velocity MotionPlanner::getClippedDribbleVelocity(const Velocity& requestedVelocity) const
{
  const auto absoluteRequestedVelocity =
      requestedVelocity.getAbsolute(walkingEngineWalkOutput_->maxVelocityComponents);
  const float clippedDribbleVelocity =
      std::min(carefulDribbleSpeed_(), absoluteRequestedVelocity.translation.norm());
  const auto walkDirection = requestedVelocity.translation.normalized();
  return {walkDirection * clippedDribbleVelocity, absoluteRequestedVelocity.rotation, false};
}

void MotionPlanner::setWalkBehindBallPosition(float offsetRotationAngle)
{
  // First, the *current* value of the walkData.target gets read
  // and is interpreted as a kickPose attached to the ball.
  const Pose& kickPose = motionRequest_->walkData.target;
  const Vector2f& ballPosition = ballState_->position;
  const Vector2f& absballSource = robotPosition_->robotToField(ballState_->position);
  const Vector2f& absBallTarget = motionPlannerOutput_->kickData.ballDestination;
  // Calculate the angle between the ball/robot-line and the direction
  // where the ball should go (indicated by the walkTarget/kickPose orientation)
  const float robot2BallAngle = std::atan2(ballPosition.y(), ballPosition.x());
  const float robot2BallTargetAngle =
      Angle::normalizeAngleDiff(robot2BallAngle - kickPose.orientation);
  // The ballTargetDirection is the direction vector pointing to where the ball should move to.
  const Vector2f ballTargetDirection(std::cos(kickPose.orientation),
                                     std::sin(kickPose.orientation));
  // Get a reference to the current walking target to modify it with an offset
  Pose& offsetTarget = motionPlannerOutput_->walkData.target;
  // Constantly apply an offset to the walk target, as long as the targetReached flag is
  // unset (which means the offset hasn't been reached yet; robot isn't close to the ball).
  // The target should be set a offset position that moved back , but it should be set a little
  // bit closer to the side the robot is coming from
  if (!offsetBallTargetReached_)
  {
    // This angle specifies how much the offset target is rotated
    // towards the robot, regardless of the direction that is determined later.
    offsetRotationAngle = std::abs(offsetRotationAngle);

    // Aim for the ball if walking around is required
    if (std::abs(robot2BallTargetAngle) > walkAroudBallAngleThreshold_())
    {
      offsetTarget.position = ballPosition.normalized();
    }
    // Only if the robot is in a certain angle region, rotate the offset position along
    // the ball radius towards the robot. This decreases excessive detouring.
    else if (std::abs(robot2BallTargetAngle) > offsetRotationAngle)
    {
      const float detBallBallTarget =
          (robotPosition_->pose.position - absballSource).x() *
              (absBallTarget - robotPosition_->robotToField(ballState_->position)).y() -
          ((robotPosition_->pose.position - absballSource).y()) *
              (absBallTarget - robotPosition_->robotToField(ballState_->position)).x();
      // Line that connects robot position and ball position
      // Make sure to correctly rotate the walk target towards the left/right side of the field,
      // depending on how the robot and the ball are positioned relative to each other.
      if (detBallBallTarget < 0)
      {
        offsetRotationAngle *= -1;
      }
      // Calculate the offset walk target position and also rotate it a little towards the robot.
      // This makes the robot already aim for a path around the ball from farther away.
      offsetTarget.position = ballPosition - Rotation2Df(offsetRotationAngle) *
                                                 ballTargetDirection * ballOffsetDistance_();
    }
    else
    {
      // "Pull" the walk target back as above, but don't rotate it towards the robot
      // since it is already inside an angle region in front of the ball.
      offsetTarget.position = ballPosition - (ballTargetDirection * ballOffsetDistance_());
    }
  }

  // WalkAroundBallPose_ is used to walk around a ball in a circle while facing it until the offset
  // walk target is reached
  if (!walkAroundBallTargetReached_)
  {
    const int sign = robot2BallTargetAngle < 0.f ? 1 : -1;
    walkAroundBallPose_.orientation = robot2BallAngle + 30 * TO_RAD * static_cast<float>(sign);
    walkAroundBallPose_.position =
        Vector2f(ballState_->position.y(), -ballState_->position.x()).normalized() *
        static_cast<float>(sign);
  }
  else
  {
    walkAroundBallPose_ = Pose();
  }

  // Determine if the ball obstacle should be ignored.
  // It should be ignored if the robot is on the correct side to avoid complications
  // while dribbling. The robot is on the correct side if it is in the half-plane
  // behind the ball away from the enemy side.
  if (std::abs(robot2BallTargetAngle) <= 90 * TO_RAD)
  {
    ignoreBallObstacle_ = true;
  }
  else if (std::abs(robot2BallTargetAngle) > 95 * TO_RAD) // Hysteresis
  {
    ignoreBallObstacle_ = false;
  }

  // When the ball is close to a robot we want to ignore the robot obstacle if we are about to
  // dribble/kick.
  const float ignoreRobotObstacleRadius =
      3.0f * obstacleData_->typeRadius[static_cast<int>(ObstacleType::HOSTILE_ROBOT)];
  if (ignoreBallObstacle_ && ballPosition.norm() <= ignoreRobotObstacleRadius)
  {
    ignoreRobotObstacles_ = true;
  }
  else if (!ignoreBallObstacle_ ||
           ballPosition.norm() > ignoreRobotObstacleRadius)
  {
    ignoreRobotObstacles_ = false;
  }

  // In the following, determine if the flag should set by checking if the robot is properly
  // aligned behind the ball. A cone is placed behind the ball and checks
  // are performed to see if the robot is inside the cone and if the robot's
  // orientation matches the target pose's orientation to some degree (pun intended).
  // A hysteresis is used for resetting.

  // Place the apex for the cone check at an offset behind the kick pose or the ball so that
  // it creates a specific opening at the kick pose position. 15cm seems reasonable for now.
  const float opening = 0.15;
  const Vector2f xOffset = (opening / std::tan(dribblingAngleTolerance_())) * ballTargetDirection;
  Vector2f apex;
  if (motionPlannerOutput_->walkData.mode == WalkMode::DRIBBLE)
  {
    apex = ballPosition + xOffset;
  }
  else
  {
    apex = kickPose.position + xOffset;
  }
  // Calculate the angle on the cone between the robot position and the cone axis
  const float axisAngle = Angle::angleDiff(std::atan2(apex.y(), apex.x()), kickPose.orientation);
  // Set the tolerance for the distance check to the
  // hybridAlignDistance to prevent aligning to the offset target
  const float distanceTolerance = hybridAlignDistance_();
  // distance of robot to line between ball source and target
  const float distToBallTargetLine = Geometry::distPointToLine(absballSource, absBallTarget, robotPosition_->pose.position);
  if (!offsetBallTargetReached_)
  {
    if (offsetTarget.position.norm() <= distanceTolerance &&
        std::abs(kickPose.orientation) < ballOffsetTargetOrientationTolerance_() &&
        axisAngle <= dribblingAngleTolerance_())
    {
      if (motionPlannerOutput_->walkData.mode == WalkMode::DRIBBLE)
      {
        // only consider distToBallTargetLine if dribbling
        if (distToBallTargetLine < maxDistToBallTargetLine_())
        {
          offsetBallTargetReached_ = true;
        }
      }
      else
      {
        offsetBallTargetReached_ = true;
      }
    }
  }
  else
  {
    // Hysteresis to reset the flag based on angle deviation, specifically for dribbling.
    const float angleHysteresis = 5 * TO_RAD;
    // Hysteresis to reset the flag based on distance.
    const float distanceHysteresis = 0.1;
    if (offsetTarget.position.norm() > distanceTolerance + distanceHysteresis ||
        std::abs(kickPose.orientation) >
            ballOffsetTargetOrientationTolerance_() + angleHysteresis ||
        axisAngle > dribblingAngleTolerance_() + angleHysteresis)
    {
      offsetBallTargetReached_ = false;
    }
    if (motionPlannerOutput_->walkData.mode == WalkMode::DRIBBLE)
    {
      // only consider distToBallTargetLine if dribbling
      if (distToBallTargetLine >= maxDistToBallTargetLine_() + distanceHysteresis)
      {
        offsetBallTargetReached_ = false;
      }
    }
  }

  // Don't walk around ball if offset target is reached
  if (offsetBallTargetReached_)
  {
    walkAroundBallTargetReached_ = true;
  }
  else
  {
    // Update walkAroundBallTargetReached_ based on distance to ball and angle between robot-ball
    // and ball-target direction
    const float distanceToBall = ballState_->position.norm();
    if (walkAroundBallTargetReached_)
    {
      if (distanceToBall <= walkAroundBallDistanceThreshold_() &&
          std::abs(robot2BallTargetAngle) >= walkAroudBallAngleThreshold_())
      {
        walkAroundBallTargetReached_ = false;
      }
    }
    else
    {
      // Hysteresis to reset the flag based on distance.
      const float distanceHysteresis = 0.1;
      // Hysteresis to reset the flag based on angle deviation
      const float angleHysteresis = 5 * TO_RAD;
      if (distanceToBall > walkAroundBallDistanceThreshold_() + distanceHysteresis ||
          std::abs(robot2BallTargetAngle) < walkAroudBallAngleThreshold_() - angleHysteresis)
      {
        walkAroundBallTargetReached_ = true;
      }
    }
  }
}

float MotionPlanner::calculateRotation() const
{
  assert(motionPlannerOutput_->walkData.mode != WalkMode::VELOCITY);
  switch (motionPlannerOutput_->walkData.mode)
  {
    case WalkMode::PATH_WITH_ORIENTATION:
    case WalkMode::DIRECT_WITH_ORIENTATION:
      // Use the target orientation during the whole path in these modes
      return Angle::normalized(motionPlannerOutput_->walkData.target.orientation);
    case WalkMode::WALK_BEHIND_BALL:
    {
      if (offsetBallTargetReached_)
      {
        return interpolatedAngle(targetAlignDistance_());
      }
      if (!walkAroundBallTargetReached_)
      {
        return walkAroundBallPose_.orientation;
      }
      return interpolatedAngle(targetAlignDistance_());
    }
    case WalkMode::DRIBBLE:
      // While dribbling, align earlier to the real walk target orientation after having reached the
      // offset target.
      if (offsetBallTargetReached_)
      {
        return interpolatedAngle(dribbleAlignDistance_());
      }
      if (!walkAroundBallTargetReached_)
      {
        return walkAroundBallPose_.orientation;
      }
      return interpolatedAngle(targetAlignDistance_());

    default:
      return interpolatedAngle(targetAlignDistance_());
  }
}

Vector2f MotionPlanner::calculateTranslation()
{
  assert(motionPlannerOutput_->walkData.mode != WalkMode::VELOCITY);
  const Pose& walkTarget = motionPlannerOutput_->walkData.target;
  const float velocityLimit = motionRequest_->walkData.velocity.translation.norm();

  switch (motionPlannerOutput_->walkData.mode)
  {
    case WalkMode::DIRECT:
    case WalkMode::DIRECT_WITH_ORIENTATION:
    {
      // If a direct walking mode was specified, no obstacles avoidance happens
      // and a normalized vector pointing to the target gets returned
      Vector2f outputVector = walkTarget.position;
      return outputVector.normalized() * velocityLimit;
    }
    case WalkMode::DRIBBLE:
    {
      if (offsetBallTargetReached_)
      {
        // Walk directly at the ball, ignoring the obstacles.
        return dribblingDirection() * velocityLimit;
      }
      if (!walkAroundBallTargetReached_)
      {
        return walkAroundBallPose_.position * velocityLimit;
      }
      return obstacleAvoidanceVector() * velocityLimit;
    }
    case WalkMode::WALK_BEHIND_BALL:
    {
      if (offsetBallTargetReached_)
      {
        // We have reached the forward ball target, now move directly to the ball target
        // and ignore obstacles
        Vector2f outputVector = walkTarget.position.normalized();
        // Factor in the slowBallApproach parameter to avoid overshooting the target pose
        return outputVector * velocityLimit * slowBallApproachFactor_();
      }
      if (!walkAroundBallTargetReached_)
      {
        return walkAroundBallPose_.position * velocityLimit;
      }
      return obstacleAvoidanceVector() * velocityLimit;
    }
    default:
    {
      return obstacleAvoidanceVector() * velocityLimit;
    }
  }
}

Vector2f MotionPlanner::dribblingDirection()
{
  // Calculate to a position 5cm to the side of the ball. Return a normalized vector pointing
  // to this position, so that the robot hits the ball with one of his feet while walking towards
  // that position.
  const Vector2f& relballSource = ballState_->position;
  const Vector2f& absballSource = robotPosition_->robotToField(ballState_->position);
  const Vector2f& absBallTarget = motionPlannerOutput_->kickData.ballDestination;
  const Vector2f& relBallTarget =
      robotPosition_->fieldToRobot(motionPlannerOutput_->kickData.ballDestination);

  // Calculate a short vector with the same direction as the line connecting ball and ball-target
  const Vector2f normalizedBallDirection =
      (relBallTarget - relballSource).normalized() * footOffset_();
  // Turn the previous vector to the side so that it is perpendicular to the ball direction
  const Vector2f footOffset(normalizedBallDirection.y(), -normalizedBallDirection.x());

  // Calculate final position to aim at for dribbling, which is slightly offset to the left or right
  // side of the ball. counter for reducing update frequency, in order to reduce time standing in
  // front of the ball
  if (cycleCounter_ % 10)
  {
    const float detBallBallTarget =
        (robotPosition_->pose.position - absballSource).x() *
            (absBallTarget - robotPosition_->robotToField(ballState_->position)).y() -
        ((robotPosition_->pose.position - absballSource).y()) *
            (absBallTarget - robotPosition_->robotToField(ballState_->position)).x();

    if (lastfootdecision_ == FootDecision::NONE)
    {
      // initial check whether the nao is left or right from the ball-target-line
      if (detBallBallTarget > 0)
      {
        lastfootdecision_ = FootDecision::LEFT;
      }
      else
      {
        lastfootdecision_ = FootDecision::RIGHT;
      }
    }
    else if (lastfootdecision_ == FootDecision::LEFT)
    {
      // checking whether the site of nao has changed compared to last time
      if (detBallBallTarget < 0)
      {
        lastfootdecision_ = FootDecision::RIGHT;
      }
    }
    else if (lastfootdecision_ == FootDecision::RIGHT)
    {
      // checking whether the site of nao has changed compared to last time
      if (detBallBallTarget > 0)
      {
        lastfootdecision_ = FootDecision::LEFT;
      }
    }
  }
  cycleCounter_++;
  // adding or subtract from initial walktarget, in order to make the nao use his left or right foot
  if (lastfootdecision_ == FootDecision::LEFT)
  {
    return Vector2f(relballSource + footOffset).normalized();
  }
  if (lastfootdecision_ == FootDecision::RIGHT)
  {
    return Vector2f(relballSource - footOffset).normalized();
  }
  return Vector2f::Zero();
}

Vector2f MotionPlanner::obstacleAvoidanceVector() const
{ // If no direct walking mode was specified, do obstacle avoidance
  Vector2f targetVec =
      motionPlannerOutput_->walkData.target.position; // Walk target might have been modified above
  targetVec.normalize(); // Get a normalized vector pointing to the target position

  // Holds the superimposed displacement from all obstacles
  Vector2f obstacleDisplacement = Vector2f::Zero();

  // iterate over all obstacles
  // select obstacle model
  const std::vector<const Obstacle*>& obstaclesPtr = getRelevantObstacles();
  for (auto obstacle : obstaclesPtr)
  {
    // Special handling to ignore the ball obstacle while dribbling,
    // depending on robot/ball-alignment and ignoring goal post obstacles if required by
    // configuration.
    if ((obstacle->type == ObstacleType::BALL && ignoreBallObstacle_) ||
        (obstacle->type == ObstacleType::GOAL_POST && ignoreGoalPostObstacles_()) ||
        ((obstacle->type == ObstacleType::ANONYMOUS_ROBOT ||
          obstacle->type == ObstacleType::HOSTILE_ROBOT ||
          obstacle->type == ObstacleType::TEAM_ROBOT ||
          obstacle->type == ObstacleType::FALLEN_ANONYMOUS_ROBOT ||
          obstacle->type == ObstacleType::FALLEN_HOSTILE_ROBOT ||
          obstacle->type == ObstacleType::FALLEN_TEAM_ROBOT) &&
         ignoreRobotObstacles_))
    {
      continue;
    }

    // Get the displacement vector for each obstacle, then scale it by the obstacle
    // weight before adding it to the superposed total displacement vector.
    try
    {
      auto type = static_cast<int>(obstacle->type);
      obstacleDisplacement += displacementVector(*obstacle) * obstacleWeights_[type];
    }
    catch (const std::exception& e)
    {
      Log(LogLevel::ERROR) << "MotionPlanner: Obstacle was ignored because of an error: "
                           << e.what() << "\n";
    }
  }
  // normalize the displacement vector;
  obstacleDisplacement.normalize();

  // Calculate a weighted combination of target vector and displacement vector
  // to get next direction for the output. Note, that while each obstacle
  // has its own configurable weight, the obstacle weight used here scales
  // the total influence of obstacle displacements.
  Vector2f outputVector = targetVec + obstacleDisplacement * totalObstacleWeight_();
  // normalize the resulting direction vector
  return outputVector.normalized();
}

std::vector<const Obstacle*> MotionPlanner::getRelevantObstacles() const
{
  std::vector<const Obstacle*> obstaclesPtr;
  const bool useOnlyLocalObstacles =
      playingRoles_->role == PlayingRole::STRIKER && strikerUsesOnlyLocalObstacles_();
  if (useOnlyLocalObstacles)
  {
    obstaclesPtr.resize(obstacleData_->obstacles.size());
    std::transform(obstacleData_->obstacles.begin(), obstacleData_->obstacles.end(),
                   obstaclesPtr.begin(),
                   [](auto& obstacle) -> const Obstacle* { return &obstacle; });
  }
  else
  {
    obstaclesPtr.resize(teamObstacleData_->obstacles.size());
    std::transform(teamObstacleData_->obstacles.begin(), teamObstacleData_->obstacles.end(),
                   obstaclesPtr.begin(),
                   [](auto& obstacle) -> const Obstacle* { return &obstacle; });
  }
  return obstaclesPtr;
}

float MotionPlanner::getMinDistToObstacleCenter(const Obstacle& obstacle) const
{
  // is this an obstacle that we can collide with on foot height or is the shoulder height the
  // critical position?
  const bool footObstacle = obstacle.type == ObstacleType::BALL;
  // we have to stay further away from obstacles that reach up to should height
  return obstacle.radius +
         (footObstacle ? groundLevelAvoidanceDistance_() : shoulderLevelAvoidanceDistance_());
}

Vector2f MotionPlanner::displacementVector(const Obstacle& obstacle) const
{
  // A positive dot product means that the obstacle and
  // the walkTarget are on the same sides of the robot, therefore
  // the obstacle might be in front of the robot and relevant for motionPlanning.
  const bool obstacleIsInFront =
      (motionPlannerOutput_->walkData.target.position.dot(obstacle.relativePosition) > 0);
  // All obstacles are modelled as circles. The robot only gets pushed away
  // from an obstacle if it is inside the obstacle's preconfigured avoidance radius.
  const bool obstacleIsNear =
      obstacle.relativePosition.norm() < getMinDistToObstacleCenter(obstacle);
  if (obstacleIsInFront && obstacleIsNear)
  {
    // The walking destination
    const Vector2<float>& targetPosition = motionRequest_->walkData.target.position;
    // Vector pointing in the direction of the obstacle
    const Vector2<float>& obstacleDirection = obstacle.relativePosition.normalized();
    // Determine the relative positioning of the obstacle and the walking destination by evaluating
    // the sign of the determinant of a matrix composed of the two position vectors.
    // detSign = 1 => obstacle left from destination; detSign = -1 => obstacle right from target
    // (detSign = 0 means parallel, left or right doesn't matter in that case)
    const float det =
        targetPosition.x() * obstacleDirection.y() - targetPosition.y() * obstacleDirection.x();
    const int detSign = det > 0 ? -1 : 1;
    // Rotate the vector pointing to the obstacle away from it. Where the correct side
    // is to rotate "away" from the obstacle is determined by the previously calculated sign.
    const float rotAngle = detSign * obstacleDisplacementAngle_();
    return Rotation2Df(rotAngle) * obstacleDirection;
  }
  else
  {
    return Vector2f::Zero();
  }
}

float MotionPlanner::interpolatedAngle(const float targetAlignDistance) const
{
  assert(hybridAlignDistance_() > targetAlignDistance);
  // Interpolate between facing the target and adopting the target orientation in other modes.
  const Pose& targetPose = motionPlannerOutput_->walkData.target;
  // The distance from robot origin to target can directly be obtained
  // from coordinates of the target pose because we are using relative coordinates.
  float distanceToTargetPose = targetPose.position.norm();

  // If the distance is to low we return the original orientation to avoid numerical problems.
  if (distanceToTargetPose < 2 * std::numeric_limits<float>::epsilon())
  {
    return targetPose.orientation;
  }

  // Case: Far away from goal -> Face the target position
  float targetFacingFactor = 0;
  if (distanceToTargetPose > hybridAlignDistance_())
  {
    targetFacingFactor = 1;
  }
  // If within goalAlignDistance,
  // or if within hybridAlignDistance AND already close to the targetPose orientation,
  // then stop facing the target position and adopt targetPose orientation directly
  else if ((distanceToTargetPose < targetAlignDistance) ||
           (distanceToTargetPose <
                targetAlignDistance + ((hybridAlignDistance_() - targetAlignDistance) / 2.f) &&
            std::abs(targetPose.orientation) < 5 * TO_RAD))
  {
    targetFacingFactor = 0;
  }
  // Else, calculate interpolation value between facing
  // the target and adopting the targetPose orientation.
  // This causes the robot to progressively align to the targetPose orientation
  // the closer it gets to the targetPose.
  else
  {
    targetFacingFactor = (distanceToTargetPose - targetAlignDistance) /
                         (hybridAlignDistance_() - targetAlignDistance);
  }
  // Interpolate between facing the target and adopting the target pose orientation,
  // to calculate the rotation angle to be achieved. To do so, angle deviations
  // are weighted according to the previously calculated targetFacingFactor.
  const Vector2f additiveTerm =
      Vector2f(std::cos(targetPose.orientation), std::sin(targetPose.orientation)) *
      (1.f - targetFacingFactor);
  const Vector2f combinedDirection =
      (targetPose.position * targetFacingFactor / distanceToTargetPose) + additiveTerm;

  return std::atan2(combinedDirection.y(), combinedDirection.x());
}

void MotionPlanner::toValue(Uni::Value& value) const
{
  value = Uni::Value(Uni::ValueType::OBJECT);

  // The walkData velocity always contains the translation to apply instantly
  value["translation"] << motionPlannerOutput_->walkData.velocity.translation;
  // The walkData.target always contains the relative orientation to achieve instantly, regardless
  // of the mode the motionPlanner is in. (while walkData.velocity.rotation is mode dependent)
  value["rotation"] << motionPlannerOutput_->walkData.target.orientation;
  // Show if the offset walk target near the ball has been reached
  value["offsetBallTargetReached"] << offsetBallTargetReached_;
  // Send the current target pose the robot tries to reach.
  // The walkTarget orientation from the motionRequest is being used rather than the one from
  // from the motionPlannerOutput because the motionPlannerOutput contains the
  // orientation to apply instantly while the motionRequest contains the final
  // orientation to be achieved at the target position.
  value["walkTarget"] << Pose(motionPlannerOutput_->walkData.target.position,
                              motionRequest_->walkData.target.orientation);
}
