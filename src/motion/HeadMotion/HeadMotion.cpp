#include <cmath>

#include "Modules/NaoProvider.h"
#include "Tools/Kinematics/ForwardKinematics.h"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Range.hpp"

#include "HeadMotion.hpp"


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
  , motionRequest_(*this)
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
  , jointAngles_({0.f, 0.f})
{
  shoulderCoverYawAngle_() *= TO_RAD;
}

void HeadMotion::cycle()
{
  filterSensorData();

  if (motionActivation_->headCanBeUsed &&                              //
      (motionRequest_->headMotion == MotionRequest::HeadMotion::ANGLES //
       || motionRequest_->headMotion == MotionRequest::HeadMotion::LOOK_AT))
  {
    if (motionRequest_->headMotion == MotionRequest::HeadMotion::ANGLES)
    {
      // The angles or head yaw and pitch can be directly taken from the head data
      requestedHeadYaw_ = motionRequest_->headAngleData.headYaw;
      requestedHeadPitch_ = motionRequest_->headAngleData.headPitch;
      requestedHeadYawVelocity_ = motionRequest_->headAngleData.maxHeadYawVelocity;
      requestedHeadPitchVelocity_ = motionRequest_->headAngleData.maxHeadPitchVelocity;
      useEffectiveYawVelocity_ = motionRequest_->headAngleData.useEffectiveYawVelocity;
    }
    else if (motionRequest_->headMotion == MotionRequest::HeadMotion::LOOK_AT)
    {
      // The head data only contains a target to look at, thus head yaw and pitch have to be
      // calculated first
      selectCameraAndAnglesForTarget(motionRequest_->headLookAtData.targetPosition);
      requestedHeadYawVelocity_ = motionRequest_->headAngleData.maxHeadYawVelocity;
      requestedHeadPitchVelocity_ = motionRequest_->headAngleData.maxHeadPitchVelocity;
      useEffectiveYawVelocity_ = false;
    }
    calculateJointAnglesFromRequest();
  }
  else
  {
    // if head can not be used (e.g. fallen) use some more stiffness hold the angles
    headMotionOutput_->angles = jointAngles_;
    headMotionOutput_->stiffnesses = {0.8f, 0.8f};
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

std::vector<float> HeadMotion::calculateHeadAnglesFromTarget(const Vector3f& targetPosition,
                                                             const KinematicMatrix& cam2head,
                                                             float yawMax) const
{
  const KinematicMatrix cam2ground = robotKinematics_->matrices[JOINTS::TORSO2GROUND] *
                                     ForwardKinematics::getHead(std::vector<float>(
                                         JOINTS_HEAD::HEAD_MAX, 0))[JOINTS_HEAD::HEAD_PITCH] *
                                     cam2head;

  // KinematicMatrices use millimeters, thus the multiplication by 1000.
  Vector3f dest2cam(
      cam2ground.invert() *
      (Vector3f(targetPosition.x(), targetPosition.y(), targetPosition.z()) * 1000.f));
  float headYaw = std::atan2(dest2cam.y(), dest2cam.x());

  // Limit head yaw:
  headYaw = Range<float>::clipToGivenRange(headYaw, -yawMax, yawMax);

  float headPitch = -std::atan2(dest2cam.z(), dest2cam.x());

  return {headYaw, headPitch};
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
  float yawDiff = requestedHeadYaw_ - jointAngles_[JOINTS_HEAD::HEAD_YAW];
  float pitchDiff = requestedHeadPitch_ - jointAngles_[JOINTS_HEAD::HEAD_PITCH];
  const float yawDirection = yawDiff > 0 ? 1 : -1;

  // get requested maximal velocities (or defaults if no velocity is specified)
  const float desiredYawVel =
      ((requestedHeadYawVelocity_ > 0 && requestedHeadYawVelocity_ <= maxYawVelocity_())
           ? requestedHeadYawVelocity_
           : maxYawVelocity_()) *
      yawDirection;

  // The negative angular velocity of the torso (yaw) is added to the requested
  // velocity.
  const bool coveredByShoulder =
      std::abs(jointAngles_[JOINTS_HEAD::HEAD_YAW]) > shoulderCoverYawAngle_();
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
  const float maxYawStep = compensatedYawVel * cycleInfo_->cycleTime;
  yawDiff = compensatedYawVel < 0 ? Range<float>::clipToGivenRange(yawDiff, maxYawStep, 0.f)
                                  : Range<float>::clipToGivenRange(yawDiff, 0.f, maxYawStep);

  const float absMaxPitchStep = pitchVel * cycleInfo_->cycleTime;
  pitchDiff = Range<float>::clipToGivenRange(pitchDiff, -absMaxPitchStep, absMaxPitchStep);

  // calculated targeted head yaw and pitch with computed difference
  float headYawTarget = jointAngles_[JOINTS_HEAD::HEAD_YAW] + yawDiff;
  float headPitchTarget = jointAngles_[JOINTS_HEAD::HEAD_PITCH] + pitchDiff;
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
    jointAngles_[JOINTS_HEAD::HEAD_PITCH] = upperPitchLimit;
    pitchWasLimited = true;
  }
  else if (headPitchTarget < 0)
  {
    jointAngles_[JOINTS_HEAD::HEAD_PITCH] = 0;
    pitchWasLimited = true;
  }
  else
  {
    jointAngles_[JOINTS_HEAD::HEAD_PITCH] = headPitchTarget;
  }
  // limit head yaw if necessary
  const float maxHeadYaw = NaoProvider::maxRange(JOINTS::HEAD_YAW);
  if (headYawTarget > maxHeadYaw)
  {
    jointAngles_[JOINTS_HEAD::HEAD_YAW] = maxHeadYaw;
    yawWasLimited = true;
  }
  else if (headYawTarget < -maxHeadYaw)
  {
    jointAngles_[JOINTS_HEAD::HEAD_YAW] = -maxHeadYaw;
    yawWasLimited = true;
  }
  else
  {
    jointAngles_[JOINTS_HEAD::HEAD_YAW] = headYawTarget;
  }
  // fill output data type
  headMotionOutput_->angles = jointAngles_;
  headMotionOutput_->stiffnesses = {0.4f, 0.7f};
  if ((std::abs(requestedHeadYaw_ - jointAngles_[JOINTS_HEAD::HEAD_YAW]) +
       std::abs(requestedHeadPitch_ - jointAngles_[JOINTS_HEAD::HEAD_PITCH])) < 0.01 ||
      (pitchWasLimited &&
       (std::abs(requestedHeadYaw_ - jointAngles_[JOINTS_HEAD::HEAD_YAW]) < 0.01)) ||
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
  const float yawMax = NaoProvider::maxRange(JOINTS::HEAD_YAW);
  const std::vector<float> currentHeadAngles = jointSensorData_->getHeadAngles();

  // Calculate the joint angles for both top and bottom camera
  const std::vector<float> topCamAngles =
      calculateHeadAnglesFromTarget(targetPosition, topCam2head, yawMax);
  const std::vector<float> bottomCamAngles =
      calculateHeadAnglesFromTarget(targetPosition, bottomCam2head, yawMax);

  // Select the angles that require less movement of the head
  if (std::abs(topCamAngles[1] - currentHeadAngles[1]) <
      std::abs(bottomCamAngles[1] - currentHeadAngles[1]))
  {
    requestedHeadYaw_ = topCamAngles[JOINTS_HEAD::HEAD_YAW];
    requestedHeadPitch_ = topCamAngles[JOINTS_HEAD::HEAD_PITCH];
  }
  else
  {
    requestedHeadYaw_ = bottomCamAngles[JOINTS_HEAD::HEAD_YAW];
    requestedHeadPitch_ = bottomCamAngles[JOINTS_HEAD::HEAD_PITCH];
  }
}
