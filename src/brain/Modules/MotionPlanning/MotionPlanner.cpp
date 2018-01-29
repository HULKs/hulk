#include <cmath>
#include <limits>
#include <stdexcept>

#include "Data/MotionRequest.hpp"
#include "Tools/Math/Angle.hpp"
#include "print.h"

#include "MotionPlanner.hpp"

MotionPlanner::MotionPlanner(const ModuleManagerInterface& manager)
  : Module(manager, "MotionPlanner")
  , enableWalkingModes_(*this, "enableWalkingModes", [] {})
  , hybridAlignDistance_(*this, "hybridAlignDistance", [] {})
  , targetAlignDistance_(*this, "targetAlignDistance", [] {})
  , ballOffsetShiftAngle_(*this, "ballOffsetShiftAngle", [this] { ballOffsetShiftAngle_() *= TO_RAD; })
  , obstacleWeight_(*this, "obstacleWeight", [] {})
  , ballDisplacementAngle_(*this, "ballDisplacementAngle", [this] { ballDisplacementAngle_() *= TO_RAD; })
  , ballAvoidanceRadius_(*this, "ballAvoidanceRadius", [] {})
  , ballWeight_(*this, "ballWeight", [] {})
  , sonarDisplacementAngle_(*this, "sonarDisplacementAngle", [this] { sonarDisplacementAngle_() *= TO_RAD; })
  , sonarAvoidanceRadius_(*this, "sonarAvoidanceRadius", [] {})
  , sonarWeight_(*this, "sonarWeight", [] {})
  , motionRequest_(*this)
  , obstacleData_(*this)
  , robotPosition_(*this)
  , ballState_(*this)
  , motionPlannerOutput_(*this)
  , offsetBallTargetReached_(false)
{
  ballOffsetShiftAngle_() *= TO_RAD;
  ballDisplacementAngle_() *= TO_RAD;
  sonarDisplacementAngle_() *= TO_RAD;
  if (!obstacleWeight_())
  {
    print("MotionPlanner obstacle weight was initialized to 0, all obstacles will be ignored.", LogLevel::WARNING);
  }
}

void MotionPlanner::cycle()
{
  // Copy current MotionRequest to MotionPlannerOutput, this way the motionPlanner may modify the request to pass on its results in form of
  // its own output, without modifying the original request. '&*' is needed because the dependency is not a real pointer.
  motionRequest_->copy(&*motionPlannerOutput_);
  // Only perform motionplanning when the robot is walking
  if (motionRequest_->bodyMotion != MotionRequest::BodyMotion::WALK)
  {
    return;
  }
  if (!enableWalkingModes_())
  {
    // Fall back to default path mode
    motionPlannerOutput_->walkData.mode = WalkMode::PATH;
  }
  // When not in velocity mode, calculate the desired rotation and translation
  if (motionPlannerOutput_->walkData.mode != WalkMode::VELOCITY)
  {
    if (motionPlannerOutput_->walkData.mode == WalkMode::WALK_BEHIND_BALL)
    {
      // Sets a new offset target away from the ball
      setWalkBehindBallPosition(std::abs(ballOffsetShiftAngle_()));
    }
    else if (motionPlannerOutput_->walkData.mode == WalkMode::DRIBBLE)
    {
      // TODO Replace magic number
      setWalkBehindBallPosition(2 * TO_RAD);
    }
    // Calculate the orientation the robot should achieve.
    motionPlannerOutput_->walkData.target.orientation = calculateRotation();
    // Calculate the next translation vector (just a direction), with or without weighted obstacle displacement
    Vector2f translationVector = calculateTranslation();
    // Scale normed translation vector with specified translation velocity
    motionPlannerOutput_->walkData.velocity.translation = translationVector * motionPlannerOutput_->walkData.velocity.translation.norm();
    // In DRIBBLE mode, change the walking mode to velocity after reaching the offset target waypoint,
    // to avoid braking when getting near the ball
    if (motionPlannerOutput_->walkData.mode == WalkMode::DRIBBLE && offsetBallTargetReached_)
    {
      // Copy the target orientation to the velocity because it is needed in velocity mode
      motionPlannerOutput_->walkData.velocity.rotation = motionPlannerOutput_->walkData.target.orientation;
      motionPlannerOutput_->walkData.mode = WalkMode::VELOCITY;
    }
  }
  // The key is for compatibility reasons.
  debug().update("Motion.Pendulum.targetPose", motionPlannerOutput_->walkData.target);
  // Send debug data
  debug().update(mount_, *this);
}

void MotionPlanner::setWalkBehindBallPosition(float offsetRotationAngle)
{
  // In this walking mode, the walk target will always be a target behind the ball
  const Pose& walkTarget = motionPlannerOutput_->walkData.target;
  const Vector2f& ballPosition = ballState_->position;
  const float distanceHysteresis = 0.1;                             // Hysteresis to reset offsetBallTargetReached_ flag.
  const float distanceTolerance = hybridAlignDistance_();           // this prevents aligning to the offset target
  float rotatedBallWalkTargetAngle = std::abs(offsetRotationAngle); // just temp for change this value during runtime.

  // Constantly apply an offset to the walk target, as long as the targetReached flag is
  // unset (which means the offset hasn't been reached yet; robot isn't close to the ball).
  if (!offsetBallTargetReached_)
  {
    // The target should be set a offset position that moved back in the orientation
    // direction, but it should be set a little bit closer to the side the robot is
    // coming from calc offset walk target behind the ball
    const Vector2f ballTargetDirection(std::cos(walkTarget.orientation), std::sin(walkTarget.orientation));
    const Vector2f robotToBallVec = robotPosition_->robotToField(ballPosition) - robotPosition_->pose.position;
    // Only if the robot is in a certain angle region, rotate the offset position along
    // the ball radius towards the robot. This decreases excessive detouring.
    const float robot2BallTargetAngle =
        Angle::angleDiff(std::atan2(robotToBallVec.y(), robotToBallVec.x()), robotPosition_->robotToField(walkTarget).orientation);
    if (robot2BallTargetAngle > rotatedBallWalkTargetAngle)
    {
      // Make sure to correctly rotate the walk target behind the ball towards the left/right side
      // of the field, depending on how the robot and the ball are positioned relative to each other.
      if (robotToBallVec.y() < 0)
      {
        rotatedBallWalkTargetAngle *= -1;
      }
      // "Pull" the walk target back from the ball position to the perimeter of the obstacle. Also rotate it towards the robot.
      motionPlannerOutput_->walkData.target.position =
          ballPosition - (Rotation2Df(rotatedBallWalkTargetAngle) * ballTargetDirection * ballAvoidanceRadius_()); // go to offset position
    }
    else
    {
      // "Pull" the walk target back as above, but don't rotate it towards the robot
      // since it is already inside an angle region in front of the ball.
      motionPlannerOutput_->walkData.target.position = ballPosition - (ballTargetDirection * ballAvoidanceRadius_()); // go to offset position
    }
  }
  // Determine if the flag should set, by checking if the offset target
  // has been reached. A hysteresis is used for resetting.
  if (walkTarget.position.norm() <= distanceTolerance)
  {
    offsetBallTargetReached_ = true;
  }
  else if (walkTarget.position.norm() > distanceTolerance + distanceHysteresis)
  {
    offsetBallTargetReached_ = false;
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
    default:
      return getInterpolateAngle();
  }
}

Vector2f MotionPlanner::calculateTranslation()
{
  assert(motionPlannerOutput_->walkData.mode != WalkMode::VELOCITY);
  const Pose& walkTarget = motionPlannerOutput_->walkData.target;

  switch (motionPlannerOutput_->walkData.mode)
  {
    case WalkMode::DIRECT:
    case WalkMode::DIRECT_WITH_ORIENTATION:
    {
      // If a direct walking mode was specified, no obstacles avoidance happens
      // and a normalized vector pointing to the target gets returned
      Vector2f outputVector = walkTarget.position;
      return outputVector.normalized();
    }
    case WalkMode::DRIBBLE:
    {
      if (offsetBallTargetReached_)
      {
        // We have reached the offset target waypoint, now move
        // directly towards the ball and ignore obstacles
        Vector2f outputVector = ballState_->position;
        return outputVector.normalized();
      }
      return getObstacleAvoidanceVector();
    }
    case WalkMode::WALK_BEHIND_BALL:
    {
      if (offsetBallTargetReached_)
      {
        // We have reached the forward ball target, now move directly to the ball target
        // and ignore obstacles
        Vector2f outputVector = walkTarget.position;
        return outputVector.normalized();
      }
      return getObstacleAvoidanceVector();
    }
    default:
    {
      return getObstacleAvoidanceVector();
    }
  }
}

Vector2f MotionPlanner::getObstacleAvoidanceVector() const
{                                                                      // If no direct walking mode was specified, do obstacle avoidance
  Vector2f targetVec = motionPlannerOutput_->walkData.target.position; // Walk target might have been modified above
  targetVec.normalize();                                               // Get a normalized vector pointing to the target position

  // Holds the superimposed displacement from all obstacles
  Vector2f obstacleDisplacement = Vector2f::Zero();

  // iterate over all obstacles
  for (auto& obstacle : obstacleData_->obstacles)
  {
    // Get the displacement vector for each obstacle, then scale it by the obstacle
    // weight before adding it to the superposed total displacement vector.
    try
    {
      obstacleDisplacement += displacementVectorOf(obstacle) * weightOf(obstacle);
    }
    catch (const std::exception& e)
    {
      Log(LogLevel::ERROR) << "MotionPlanner: Obstacle was ignored because of an error: " << e.what() << "\n";
    }
  }
  // normalize the displacement vector;
  obstacleDisplacement.normalize();

  // Calculate a weighted combination of target vector and displacement vector
  // to get next direction for the output. Note, that while each obstacle
  // has its own configurable weight, the obstacle weight used here scales
  // the total influence of obstacle displacements.
  Vector2f outputVector = targetVec + obstacleDisplacement * obstacleWeight_();
  // normalize the resulting direction vector
  return outputVector.normalized();
}

Vector2f MotionPlanner::displacementVectorOf(const Obstacle& obstacle) const
{
  // All obstacles are modelled as circles. The robot only gets pushed away
  // from an obstacle if it is inside the obstacle's preconfigured avoidance radius.
  if (obstacle.position.norm() < avoidanceRadiusOf(obstacle))
  {
    // normalize the distance vector
    Vector2f normedPos = obstacle.position;
    normedPos.normalize();

    // Check the y coordinate of the obstacle. If > 0 (on left side) push
    // right around the obstacle, otherwise push left
    const float turnFactor = (normedPos.y() > 0 ? 1.f : -1.f);

    // compute angle of displacement
    const float alpha = turnFactor * displacementAngleOf(obstacle) + static_cast<float>(M_PI);

    // Rotate the normalized distance vector according to the calculated displacement angle
    return Rotation2Df(alpha) * normedPos;
  }
  else
  {
    return Vector2f::Zero();
  }
}

float MotionPlanner::displacementAngleOf(const Obstacle& obstacle) const
{
  switch (obstacle.type)
  {
    case Obstacle::BALL:
      return ballDisplacementAngle_();
    case Obstacle::SONAR:
      return sonarDisplacementAngle_();
    default:
      throw ObstacleTypeError();
  }
}

float MotionPlanner::avoidanceRadiusOf(const Obstacle& obstacle) const
{
  switch (obstacle.type)
  {
    case Obstacle::BALL:
      return ballAvoidanceRadius_();
    case Obstacle::SONAR:
      return sonarAvoidanceRadius_();
    default:
      throw ObstacleTypeError();
  }
}

float MotionPlanner::weightOf(const Obstacle& obstacle) const
{
  switch (obstacle.type)
  {
    case Obstacle::BALL:
      return ballWeight_();
    case Obstacle::SONAR:
      return sonarWeight_();
    default:
      throw ObstacleTypeError();
  }
}

float MotionPlanner::getInterpolateAngle() const
{
  assert(hybridAlignDistance_() > targetAlignDistance_());
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
  else if ((distanceToTargetPose < targetAlignDistance_()) ||
           (distanceToTargetPose < targetAlignDistance_() + ((hybridAlignDistance_() - targetAlignDistance_()) / 2.f) &&
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
    targetFacingFactor = (distanceToTargetPose - targetAlignDistance_()) / (hybridAlignDistance_() - targetAlignDistance_());
  }
  // Interpolate between facing the target and adopting the target pose orientation,
  // to calculate the rotation angle to be achieved. To do so, angle deviations
  // are weighted according to the previously calculated targetFacingFactor.
  const Vector2f additiveTerm = Vector2f(std::cos(targetPose.orientation), std::sin(targetPose.orientation)) * (1.f - targetFacingFactor);
  const Vector2f combinedDirection = (targetPose.position * targetFacingFactor / distanceToTargetPose) + additiveTerm;

  return std::atan2(combinedDirection.y(), combinedDirection.x());
}

void MotionPlanner::toValue(Uni::Value& value) const
{
  value = Uni::Value(Uni::ValueType::OBJECT);
  value["obstacles"] << obstacleData_->obstacles;

  // Create and serialize a vector of the obstacle radii
  std::vector<float> avoidanceRadii;
  for (auto& obstacle : obstacleData_->obstacles)
  {
    avoidanceRadii.push_back(avoidanceRadiusOf(obstacle));
  }
  value["avoidanceRadii"] << avoidanceRadii;
}
