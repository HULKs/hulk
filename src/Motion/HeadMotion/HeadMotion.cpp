#include "Motion/HeadMotion/HeadMotion.hpp"
#include "Hardware/JointUtils.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Range.hpp"
#include <cmath>
#include <type_traits>

HeadMotion::HeadMotion(const ModuleManagerInterface& manager)
  : Module(manager)
  , maxYawVelocity_(*this, "maxYawVelocity", [] {})
  , maxPitchVelocity_(*this, "maxPitchVelocity", [] {})
  , outerPitchMax_(*this, "outerPitchMax", [] {})
  , innerPitchMax_(*this, "innerPitchMax", [] {})
  , yawThreshold_(*this, "yawThreshold", [] {})
  , lowPassAlphaGyro_(*this, "lowPassAlphaGyro", [] {})
  , shoulderCoverYawAngle_(*this, "shoulderCoverYawAngle",
                           [this] { shoulderCoverYawAngle_() *= TO_RAD; })
  , limitHeadPitch_(*this, "limitHeadPitch", [] {})
  , actionCommand_(*this)
  , motionActivation_(*this)
  , cycleInfo_(*this)
  , jointSensorData_(*this)
  , robotKinematics_(*this)
  , imuSensorData_(*this)
  , headMotionOutput_(*this)
  , filteredTorsoYawVelocity_(0.f)
  , requestedHeadYaw_(0.f)
  , requestedHeadPitch_(0.f)
  , requestedHeadYawVelocity_(0.f)
  , requestedHeadPitchVelocity_(0.f)
  , useEffectiveYawVelocity_(false)
  , wasActive_(false)
  , wasAtTarget_(false)
{
  shoulderCoverYawAngle_() *= TO_RAD;
}

void HeadMotion::cycle()
{
  filterSensorData();
  using MotionType = ActionCommand::Head::MotionType;

  if (motionActivation_->headCanBeUsed &&                //
      (actionCommand_->head().type == MotionType::ANGLES //
       || actionCommand_->head().type == MotionType::LOOK_AT))
  {
    if (actionCommand_->head().type == MotionType::ANGLES)
    {
      // The angles or head yaw and pitch can be directly taken from the head data
      requestedHeadYaw_ = actionCommand_->head().yaw;
      requestedHeadPitch_ = actionCommand_->head().pitch;
      requestedHeadYawVelocity_ = actionCommand_->head().maxYawVelocity;
      requestedHeadPitchVelocity_ = actionCommand_->head().maxPitchVelocity;
      useEffectiveYawVelocity_ = actionCommand_->head().useEffectiveYawVelocity;
    }
    else if (actionCommand_->head().type == MotionType::LOOK_AT)
    {
      // The head data only contains a target to look at, thus head yaw and pitch have to be
      // calculated first
      selectCameraAndAnglesForTarget(actionCommand_->head().targetPosition);
      requestedHeadYawVelocity_ = actionCommand_->head().maxYawVelocity;
      requestedHeadPitchVelocity_ = actionCommand_->head().maxPitchVelocity;
      useEffectiveYawVelocity_ = false;
    }
    calculateJointAnglesFromRequest();
  }
  else
  {
    // if head can not be used (e.g. fallen) use some more stiffness hold the angles
    JointUtils::fillHead(headMotionOutput_->angles, jointAngles_);
    headMotionOutput_->stiffnesses = {{0.8f, 0.8f}};
    wasActive_ = false;
    wasAtTarget_ = false;
    resetFilters();
  }
}

void HeadMotion::resetFilters()
{
  filteredTorsoYawVelocity_ = 0.f;
}

void HeadMotion::filterSensorData()
{
  filteredTorsoYawVelocity_ = lowPassAlphaGyro_() * filteredTorsoYawVelocity_ +
                              (1 - lowPassAlphaGyro_()) * imuSensorData_->gyroscope.z();
}

JointsHeadArray<float> HeadMotion::calculateHeadAnglesFromTarget(const Vector3f& targetPosition,
                                                                 const KinematicMatrix& cam2head,
                                                                 float yawMax) const
{
  const KinematicMatrix cam2ground = robotKinematics_->torso2ground *
                                     forwardKinematics().getHead({{0.f, 0.f}})[JointsHead::PITCH] *
                                     cam2head;

  // KinematicMatrices use millimeters, thus the multiplication by 1000.
  Vector3f dest2cam(
      cam2ground.inverted() *
      (Vector3f(targetPosition.x(), targetPosition.y(), targetPosition.z()) * 1000.f));
  float headYaw = std::atan2(dest2cam.y(), dest2cam.x());

  // Limit head yaw:
  headYaw = Range<float>::clipToGivenRange(headYaw, -yawMax, yawMax);

  float headPitch = -std::atan2(dest2cam.z(), dest2cam.x());

  return {{headYaw, headPitch}};
}

void HeadMotion::calculateJointAnglesFromRequest()
{
  // If the head motion module was not used in the previous cycle, sensor values are used as a
  // starting point.
  if (!wasActive_)
  {
    jointAngles_ = jointSensorData_->getHeadAngles();
    wasActive_ = true;
  }
  // compute the difference from the current angles to the target angles
  float yawDiff = requestedHeadYaw_ - jointAngles_[JointsHead::YAW];
  float pitchDiff = requestedHeadPitch_ - jointAngles_[JointsHead::PITCH];
  const float yawDirection = yawDiff > 0 ? 1 : -1;

  // get requested maximal velocities (or defaults if no velocity is specified)
  const float desiredYawVel =
      ((requestedHeadYawVelocity_ > 0 && requestedHeadYawVelocity_ <= maxYawVelocity_())
           ? requestedHeadYawVelocity_
           : maxYawVelocity_()) *
      yawDirection;

  // The negative angular velocity of the torso (yaw) is added to the requested
  // velocity.
  const bool coveredByShoulder = std::abs(jointAngles_[JointsHead::YAW]) > shoulderCoverYawAngle_();
  const bool deceleratingCompensation = yawDirection * filteredTorsoYawVelocity_ > 0;

  const float torsoVelocityCompensation =
      useEffectiveYawVelocity_ && !(coveredByShoulder && deceleratingCompensation)
          ? -filteredTorsoYawVelocity_
          : 0.f;

  const float compensatedYawVel = Range<float>::clipToGivenRange(
      desiredYawVel + torsoVelocityCompensation, -maxYawVelocity_(), maxYawVelocity_());

  const float pitchVel =
      ((requestedHeadPitchVelocity_ > 0 && requestedHeadPitchVelocity_ <= maxPitchVelocity_())
           ? requestedHeadPitchVelocity_
           : maxPitchVelocity_());

  // clip difference to target to the maximum distance that can be moved in one cycle
  static_assert(std::is_same_v<Clock::duration::period, std::chrono::seconds::period>);
  const float maxYawStep = compensatedYawVel * cycleInfo_->cycleTime.count();
  yawDiff = compensatedYawVel < 0 ? Range<float>::clipToGivenRange(yawDiff, maxYawStep, 0.f)
                                  : Range<float>::clipToGivenRange(yawDiff, 0.f, maxYawStep);

  const float absMaxPitchStep = pitchVel * cycleInfo_->cycleTime.count();
  pitchDiff = Range<float>::clipToGivenRange(pitchDiff, -absMaxPitchStep, absMaxPitchStep);

  // calculated targeted head yaw and pitch with computed difference
  float headYawTarget = jointAngles_[JointsHead::YAW] + yawDiff;
  float headPitchTarget = jointAngles_[JointsHead::PITCH] + pitchDiff;
  // smooth interpolation of pitch limit between yaw threshold
  float upperPitchLimit = 0;
  bool yawWasLimited = false;
  bool pitchWasLimited = false;
  if (std::fabs(headYawTarget) > yawThreshold_() * TO_RAD)
  {
    upperPitchLimit = outerPitchMax_() * TO_RAD;
  }
  else
  {
    // cosinus-shaped limit (plot it and you will see the point)
    upperPitchLimit = (outerPitchMax_() + 0.5f * (innerPitchMax_() - outerPitchMax_()) *
                                              (1 + cos(180 / yawThreshold_() * headYawTarget))) *
                      TO_RAD;
  }
  // limit head pitch if necessary (greater pitch means looking down)
  if (headPitchTarget > upperPitchLimit)
  {
    jointAngles_[JointsHead::PITCH] = upperPitchLimit;
    pitchWasLimited = true;
  }
  else if (limitHeadPitch_() && headPitchTarget < 0)
  {
    jointAngles_[JointsHead::PITCH] = 0;
    pitchWasLimited = true;
  }
  else
  {
    jointAngles_[JointsHead::PITCH] = headPitchTarget;
  }
  // limit head yaw if necessary
  const float maxHeadYaw = robotMetrics().maxRange(Joints::HEAD_YAW);
  if (headYawTarget > maxHeadYaw)
  {
    jointAngles_[JointsHead::YAW] = maxHeadYaw;
    yawWasLimited = true;
  }
  else if (headYawTarget < -maxHeadYaw)
  {
    jointAngles_[JointsHead::YAW] = -maxHeadYaw;
    yawWasLimited = true;
  }
  else
  {
    jointAngles_[JointsHead::YAW] = headYawTarget;
  }
  // fill output data type
  JointUtils::fillHead(headMotionOutput_->angles, jointAngles_);
  headMotionOutput_->stiffnesses = {{0.4f, 0.7f}};
  if ((std::abs(requestedHeadYaw_ - jointAngles_[JointsHead::YAW]) +
       std::abs(requestedHeadPitch_ - jointAngles_[JointsHead::PITCH])) < 0.01 ||
      (pitchWasLimited && (std::abs(requestedHeadYaw_ - jointAngles_[JointsHead::YAW]) < 0.01)) ||
      (pitchWasLimited && yawWasLimited))
  {
    headMotionOutput_->atTarget = true;
    headMotionOutput_->target[0] = requestedHeadYaw_;
    headMotionOutput_->target[1] = requestedHeadPitch_;
    if (!wasAtTarget_)
    {
      timeWhenReachedTarget_ = cycleInfo_->startTime;
    }
    headMotionOutput_->timeWhenReachedTarget = timeWhenReachedTarget_;
    wasAtTarget_ = true;
  }
  else
  {
    wasAtTarget_ = false;
  }
}

void HeadMotion::selectCameraAndAnglesForTarget(const Vector3f& targetPosition)
{
  const KinematicMatrix topCam2head = KinematicMatrix::transZ(63.64) *
                                      KinematicMatrix::transX(58.71) * KinematicMatrix::rotY(.0209);
  const KinematicMatrix bottomCam2head = KinematicMatrix::transZ(17.74) *
                                         KinematicMatrix::transX(50.71) *
                                         KinematicMatrix::rotY(.6929);
  const float yawMax = robotMetrics().maxRange(Joints::HEAD_YAW);
  const auto currentHeadAngles = jointSensorData_->getHeadAngles();

  // Calculate the joint angles for both top and bottom camera
  const auto topCamAngles = calculateHeadAnglesFromTarget(targetPosition, topCam2head, yawMax);
  const auto bottomCamAngles =
      calculateHeadAnglesFromTarget(targetPosition, bottomCam2head, yawMax);

  // Select the angles that require less movement of the head
  if (std::abs(topCamAngles[JointsHead::PITCH] - currentHeadAngles[JointsHead::PITCH]) <
      std::abs(bottomCamAngles[JointsHead::PITCH] - currentHeadAngles[JointsHead::PITCH]))
  {
    requestedHeadYaw_ = topCamAngles[JointsHead::YAW];
    requestedHeadPitch_ = topCamAngles[JointsHead::PITCH];
  }
  else
  {
    requestedHeadYaw_ = bottomCamAngles[JointsHead::YAW];
    requestedHeadPitch_ = bottomCamAngles[JointsHead::PITCH];
  }
}
