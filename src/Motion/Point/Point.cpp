#include "Motion/Point/Point.hpp"
#include <type_traits>

Point::Point(const ModuleManagerInterface& manager)
  : Module(manager)
  , cycleInfo_(*this)
  , actionCommand_(*this)
  , jointSensorData_(*this)
  , robotKinematics_(*this)
  , pointOutput_(*this)
{
}

void Point::cycle()
{
  if (!actionCommand_->body().usesArms() && actionCommand_->leftArm().type == MotionType::POINT &&
      actionCommand_->rightArm().type == MotionType::POINT &&
      (lastLeftArmMotion_ != MotionType::POINT || lastRightArmMotion_ != MotionType::POINT))
  {
    const Vector3f relativePoint = actionCommand_->leftArm().target;
    KinematicMatrix shoulder2torso;
    bool left = false;
    if (relativePoint.y() > 0)
    {
      // use left arm
      left = true;
      shoulder2torso = robotKinematics_->matrices[Joints::L_SHOULDER_PITCH];
    }
    else
    {
      // use right arm
      left = false;
      shoulder2torso = robotKinematics_->matrices[Joints::R_SHOULDER_PITCH];
    }
    KinematicMatrix shoulder2ground = robotKinematics_->torso2ground * shoulder2torso;
    // This vector points from the shoulder to the point.
    Vector3f direction = relativePoint - shoulder2ground.posV;
    direction.normalize();
    // Kinematics formulae figured out by @lassepe
    float shoulderRoll = std::asin(direction.y());
    float shoulderPitch = std::acos(direction.x() / std::cos(shoulderRoll));
    JointsArmArray<float> lAngles;
    JointsArmArray<float> rAngles;
    if (left)
    {
      lAngles[JointsArm::SHOULDER_PITCH] = shoulderPitch;
      lAngles[JointsArm::SHOULDER_ROLL] = shoulderRoll;
      rAngles[JointsArm::SHOULDER_PITCH] = 90 * TO_RAD;
      rAngles[JointsArm::SHOULDER_ROLL] = 0;
    }
    else
    {
      lAngles[JointsArm::SHOULDER_PITCH] = 90 * TO_RAD;
      lAngles[JointsArm::SHOULDER_ROLL] = 0;
      rAngles[JointsArm::SHOULDER_PITCH] = shoulderPitch;
      rAngles[JointsArm::SHOULDER_ROLL] = shoulderRoll;
    }
    lAngles[JointsArm::ELBOW_YAW] = -90 * TO_RAD;
    rAngles[JointsArm::ELBOW_YAW] = 90 * TO_RAD;
    lAngles[JointsArm::ELBOW_ROLL] = 0;
    rAngles[JointsArm::ELBOW_ROLL] = 0;
    lAngles[JointsArm::WRIST_YAW] = 0;
    rAngles[JointsArm::WRIST_YAW] = 0;
    lAngles[JointsArm::HAND] = 0;
    rAngles[JointsArm::HAND] = 0;
    leftInterpolator_.reset(jointSensorData_->getLArmAngles(), lAngles, 500ms);
    rightInterpolator_.reset(jointSensorData_->getRArmAngles(), rAngles, 500ms);
  }
  lastLeftArmMotion_ = actionCommand_->leftArm().type;
  lastRightArmMotion_ = actionCommand_->rightArm().type;
  // It cannot happen that leftInterpolator is finished but not rightInterpolator since both are
  // started at the same time.
  if (!leftInterpolator_.isFinished() && !rightInterpolator_.isFinished())
  {
    pointOutput_->wantToSend = true;
    pointOutput_->leftAngles = {leftInterpolator_.step(cycleInfo_->cycleTime)};
    pointOutput_->rightAngles = {rightInterpolator_.step(cycleInfo_->cycleTime)};
    pointOutput_->leftStiffnesses.fill(0.7f);
    pointOutput_->rightStiffnesses.fill(0.7f);
  }
}
