#include "Modules/NaoProvider.h"

#include "Point.hpp"

Point::Point(const ModuleManagerInterface& manager)
  : Module(manager, "Point")
  , motionRequest_(*this)
  , jointSensorData_(*this)
  , robotKinematics_(*this)
  , pointOutput_(*this)
  , leftInterpolator_()
  , rightInterpolator_()
  , lastLeftArmMotion_(MotionRequest::ArmMotion::BODY)
  , lastRightArmMotion_(MotionRequest::ArmMotion::BODY)
{
}

void Point::cycle()
{
  if (!motionRequest_->usesArms() &&
      (motionRequest_->leftArmMotion == MotionRequest::ArmMotion::POINT && motionRequest_->rightArmMotion == MotionRequest::ArmMotion::POINT) &&
      (lastLeftArmMotion_ != MotionRequest::ArmMotion::POINT || lastRightArmMotion_ != MotionRequest::ArmMotion::POINT ||
       lastPointData_.relativePoint != motionRequest_->pointData.relativePoint))
  {
    const Vector3f& relativePoint = motionRequest_->pointData.relativePoint;
    KinematicMatrix shoulder2torso;
    bool left = false;
    if (relativePoint.y() > 0)
    {
      // use left arm
      left = true;
      shoulder2torso = robotKinematics_->matrices[JOINTS::L_SHOULDER_PITCH];
    }
    else
    {
      // use right arm
      left = false;
      shoulder2torso = robotKinematics_->matrices[JOINTS::R_SHOULDER_PITCH];
    }
    KinematicMatrix shoulder2ground = robotKinematics_->matrices[JOINTS::TORSO2GROUND] * shoulder2torso;
    // This vector points from the shoulder to the point.
    Vector3f direction = relativePoint - (shoulder2ground.posV / 1000.f);
    direction.normalize();
    // Kinematics formulae figured out by @lassepe
    float shoulderRoll = std::asin(direction.y());
    float shoulderPitch = std::acos(direction.x() / std::cos(shoulderRoll));
    std::vector<float> lAngles(JOINTS_L_ARM::L_ARM_MAX);
    std::vector<float> rAngles(JOINTS_R_ARM::R_ARM_MAX);
    if (left)
    {
      lAngles[JOINTS_L_ARM::L_SHOULDER_PITCH] = shoulderPitch;
      lAngles[JOINTS_L_ARM::L_SHOULDER_ROLL] = shoulderRoll;
      rAngles[JOINTS_R_ARM::R_SHOULDER_PITCH] = 90 * TO_RAD;
      rAngles[JOINTS_R_ARM::R_SHOULDER_ROLL] = 0;
    }
    else
    {
      lAngles[JOINTS_L_ARM::L_SHOULDER_PITCH] = 90 * TO_RAD;
      lAngles[JOINTS_L_ARM::L_SHOULDER_ROLL] = 0;
      rAngles[JOINTS_R_ARM::R_SHOULDER_PITCH] = shoulderPitch;
      rAngles[JOINTS_R_ARM::R_SHOULDER_ROLL] = shoulderRoll;
    }
    lAngles[JOINTS_L_ARM::L_ELBOW_YAW] = -90 * TO_RAD;
    rAngles[JOINTS_R_ARM::R_ELBOW_YAW] = 90 * TO_RAD;
    lAngles[JOINTS_L_ARM::L_ELBOW_ROLL] = 0;
    rAngles[JOINTS_R_ARM::R_ELBOW_ROLL] = 0;
    lAngles[JOINTS_L_ARM::L_WRIST_YAW] = 0;
    rAngles[JOINTS_R_ARM::R_WRIST_YAW] = 0;
    lAngles[JOINTS_L_ARM::L_HAND] = 0;
    rAngles[JOINTS_R_ARM::R_HAND] = 0;
    leftInterpolator_.reset(jointSensorData_->getLArmAngles(), lAngles, 500);
    rightInterpolator_.reset(jointSensorData_->getRArmAngles(), rAngles, 500);
  }
  lastLeftArmMotion_ = motionRequest_->leftArmMotion;
  lastRightArmMotion_ = motionRequest_->rightArmMotion;
  lastPointData_ = motionRequest_->pointData;
  // It cannot happen that leftInterpolator is finished but not rightInterpolator since both are started at the same time.
  if (!leftInterpolator_.finished() && !rightInterpolator_.finished())
  {
    std::vector<float> lAngles = leftInterpolator_.step(10);
    std::vector<float> rAngles = rightInterpolator_.step(10);
    pointOutput_->wantToSend = true;
    pointOutput_->leftAngles = lAngles;
    pointOutput_->rightAngles = rAngles;
    pointOutput_->stiffnesses = std::vector<float>(lAngles.size() + rAngles.size(), 0.7f);
  }
}
