#include "Motion/Kick/Kick.hpp"
#include "Hardware/JointUtils.hpp"
#include "Tools/Math/Angle.hpp"
#include <type_traits>

Kick::Kick(const ModuleManagerInterface& manager)
  : Module{manager}
  , actionCommand_{*this}
  , cycleInfo_{*this}
  , imuSensorData_{*this}
  , jointSensorData_{*this}
  , kickConfigurationData_{*this}
  , motionActivation_{*this}
  , poses_{*this}
  , kickOutput_{*this}
  , torsoOffsetLeft_{*this, "torsoOffsetLeft", [] {}}
  , torsoOffsetRight_{*this, "torsoOffsetRight", [] {}}
  , gyroLowPassRatio_{*this, "gyroLowPassRatio", [] {}}
  , gyroForwardBalanceFactor_{*this, "gyroForwardBalanceFactor", [] {}}
  , gyroSidewaysBalanceFactor_{*this, "gyroSidewaysBalanceFactor", [] {}}
  , liftFootInterpolator_{&Kick::parabolicStep}
  , kickAccelerationInterpolator_{&Kick::parabolicPositiveStep}
  , kickDecelerationInterpolator_{&Kick::parabolicNegativeStep}
  , retractFootInterpolator_{&Kick::parabolicStep}
  , currentInterpolatorID_{interpolators_.size()}
  , filteredGyro_{Vector2f::Zero()}
{
}

void Kick::cycle()
{
  // update gyroscope filter
  filteredGyro_.x() = gyroLowPassRatio_() * filteredGyro_.x() +
                      (1.f - gyroLowPassRatio_()) * imuSensorData_->gyroscope.x();
  filteredGyro_.y() = gyroLowPassRatio_() * filteredGyro_.y() +
                      (1.f - gyroLowPassRatio_()) * imuSensorData_->gyroscope.y();

  // check if a kick is requested
  const bool incomingKickRequest =
      motionActivation_->activations[ActionCommand::Body::MotionType::KICK] == 1.f &&
      actionCommand_->body().type == ActionCommand::Body::MotionType::KICK;
  if (currentInterpolatorID_ == interpolators_.size() && incomingKickRequest)
  {
    // get kick kick configuration based on requested kick type
    const KickConfiguration& kickConfiguration =
        kickConfigurationData_->kicks[actionCommand_->body().kickType];
    // check whether left or right foot is to be used
    leftKicking_ = actionCommand_->body().ballPosition.y() > 0;
    // select appropriate torso offset
    const Vector3f torsoOffset = leftKicking_ ? torsoOffsetLeft_() : torsoOffsetRight_();
    // reset interpolators
    resetInterpolators(kickConfiguration, torsoOffset);
    // initialize kick
    currentInterpolatorID_ = 0;
  }

  // check whether kick if active
  if (currentInterpolatorID_ < interpolators_.size())
  {
    // do not move this check unless you want a segmentation fault
    if (interpolators_[currentInterpolatorID_]->isFinished())
    {
      // advance kick phase
      currentInterpolatorID_++;
    }
  }

  // check whether kick if active
  if (currentInterpolatorID_ < interpolators_.size())
  {
    // get output angles from current interpolator (advancing by cycle time step)
    kickOutput_->angles = {interpolators_[currentInterpolatorID_]->step(cycleInfo_->cycleTime)};
    // apply gyroscope feedback
    gyroFeedback(kickOutput_->angles);
    kickOutput_->stiffnesses.fill(0.85f);
    kickOutput_->safeExit = false;

    // mirror output angles if right foot is used
    if (!leftKicking_)
    {
      kickOutput_->mirrorAngles();
    }
  }
  else
  {
    // default kick output
    kickOutput_->angles = poses_->angles[Poses::Type::READY];
    kickOutput_->stiffnesses.fill(0.7f);
    kickOutput_->safeExit = true;
  }
}

void Kick::resetInterpolators(const KickConfiguration& kickConfiguration,
                              const Vector3f& torsoOffset)
{
  /*
   * wait before start
   */
  const auto anglesAtKickRequest = jointSensorData_->getBodyAngles();
  const auto readyPoseAngles = poses_->angles[Poses::Type::READY];
  waitBeforeStartInterpolator_.reset(anglesAtKickRequest, readyPoseAngles,
                                     kickConfiguration.waitBeforeStartDuration);

  /*
   * weight shift
   */
  const Vector3f weightShiftCom = kickConfiguration.weightShiftCom + torsoOffset;
  auto weightShiftAngles =
      computeWeightShiftAnglesFromReferenceCom(readyPoseAngles, weightShiftCom);
  weightShiftAngles[Joints::L_SHOULDER_ROLL] = kickConfiguration.shoulderRoll;
  weightShiftAngles[Joints::R_SHOULDER_ROLL] = -kickConfiguration.shoulderRoll;
  weightShiftInterpolator_.reset(readyPoseAngles, weightShiftAngles,
                                 kickConfiguration.weightShiftDuration);

  /*
   * lift foot
   */
  const float yawLeft2right = kickConfiguration.yawLeft2right;
  const KinematicMatrix liftFootPose = KinematicMatrix(AngleAxisf(yawLeft2right, Vector3f::UnitZ()),
                                                       kickConfiguration.liftFootPosition);
  auto liftFootAngles = computeLegAnglesFromFootPose(weightShiftAngles, liftFootPose);
  liftFootAngles[Joints::L_SHOULDER_PITCH] -= kickConfiguration.shoulderPitchAdjustment;
  liftFootAngles[Joints::R_SHOULDER_PITCH] += kickConfiguration.shoulderPitchAdjustment;
  liftFootAngles[Joints::L_ANKLE_ROLL] = kickConfiguration.ankleRoll;
  liftFootInterpolator_.reset(weightShiftAngles, liftFootAngles,
                              kickConfiguration.liftFootDuration);

  /*
   * kick acceleration
   */
  const KinematicMatrix kickAccelerationPose = KinematicMatrix(
      AngleAxisf(yawLeft2right, Vector3f::UnitZ()), kickConfiguration.kickAccelerationPosition);
  auto kickAccelerationAngles = computeLegAnglesFromFootPose(liftFootAngles, kickAccelerationPose);
  kickAccelerationAngles[Joints::L_SHOULDER_PITCH] += kickConfiguration.shoulderPitchAdjustment;
  kickAccelerationAngles[Joints::R_SHOULDER_PITCH] -= kickConfiguration.shoulderPitchAdjustment;
  kickAccelerationAngles[Joints::L_ANKLE_PITCH] += kickConfiguration.anklePitch;
  kickAccelerationAngles[Joints::L_ANKLE_ROLL] = kickConfiguration.ankleRoll;
  kickAccelerationInterpolator_.reset(liftFootAngles, kickAccelerationAngles,
                                      kickConfiguration.kickAccelerationDuration);

  /*
   * kick ball with constant speed
   */
  const KinematicMatrix kickConstantPose = KinematicMatrix(
      AngleAxisf(yawLeft2right, Vector3f::UnitZ()), kickConfiguration.kickConstantPosition);
  auto kickConstantAngles = computeLegAnglesFromFootPose(kickAccelerationAngles, kickConstantPose);
  kickConstantAngles[Joints::L_SHOULDER_PITCH] += kickConfiguration.shoulderPitchAdjustment;
  kickConstantAngles[Joints::R_SHOULDER_PITCH] -= kickConfiguration.shoulderPitchAdjustment;
  kickConstantAngles[Joints::L_ANKLE_ROLL] = kickConfiguration.ankleRoll;
  kickConstantInterpolator_.reset(kickAccelerationAngles, kickConstantAngles,
                                  kickConfiguration.kickConstantDuration);

  /*
   * kick deceleration
   */
  const KinematicMatrix kickDecelerationPose = KinematicMatrix(
      AngleAxisf(yawLeft2right, Vector3f::UnitZ()), kickConfiguration.kickDecelerationPosition);
  auto kickDecelerationAngles =
      computeLegAnglesFromFootPose(kickConstantAngles, kickDecelerationPose);
  kickDecelerationAngles[Joints::L_SHOULDER_PITCH] += kickConfiguration.shoulderPitchAdjustment;
  kickDecelerationAngles[Joints::R_SHOULDER_PITCH] -= kickConfiguration.shoulderPitchAdjustment;
  kickDecelerationAngles[Joints::L_ANKLE_ROLL] = kickConfiguration.ankleRoll;
  kickDecelerationInterpolator_.reset(kickConstantAngles, kickDecelerationAngles,
                                      kickConfiguration.kickDecelerationDuration);

  /*
   * retract foot
   */
  const KinematicMatrix retractFootPose = KinematicMatrix(
      AngleAxisf(yawLeft2right, Vector3f::UnitZ()), kickConfiguration.retractFootPosition);
  auto retractFootAngles = computeLegAnglesFromFootPose(kickConstantAngles, retractFootPose);
  retractFootAngles[Joints::L_SHOULDER_PITCH] -= kickConfiguration.shoulderPitchAdjustment;
  retractFootAngles[Joints::R_SHOULDER_PITCH] += kickConfiguration.shoulderPitchAdjustment;
  retractFootAngles[Joints::L_ANKLE_ROLL] = kickConfiguration.ankleRoll;
  retractFootInterpolator_.reset(kickConstantAngles, retractFootAngles,
                                 kickConfiguration.retractFootDuration);

  /*
   * extend foot and center torso
   */
  extendFootAndCenterTorsoInterpolator_.reset(retractFootAngles, readyPoseAngles,
                                              kickConfiguration.extendFootAndCenterTorsoDuration);

  /*
   * wait before exit
   */
  waitBeforeExitInterpolator_.reset(readyPoseAngles, readyPoseAngles,
                                    kickConfiguration.waitBeforeExitDuration);
}

JointsArray<float>
Kick::computeWeightShiftAnglesFromReferenceCom(const JointsArray<float>& currentAngles,
                                               const Vector3f& weightShiftCom) const
{
  auto weightShiftAngles = currentAngles;
  // iteratively move the torso to achieve the desired CoM
  for (unsigned int i = 0; i < 5; i++)
  {
    auto leftLegAngles = JointUtils::extractLeftLeg(weightShiftAngles);
    auto rightLegAngles = JointUtils::extractRightLeg(weightShiftAngles);

    KinematicMatrix com2torso{com().getCom(weightShiftAngles)};
    const KinematicMatrix right2torso = forwardKinematics().getRFoot(rightLegAngles);
    const KinematicMatrix com2right = right2torso.inverted() * com2torso;
    const KinematicMatrix left2torso = forwardKinematics().getLFoot(leftLegAngles);
    const KinematicMatrix com2left = left2torso.inverted() * com2torso;

    const Vector3f comError = com2right.posV - weightShiftCom;

    com2torso.posV += comError;

    rightLegAngles = inverseKinematics().getRLegAngles(com2torso * com2right.inverted());
    leftLegAngles = inverseKinematics().getFixedLLegAngles(
        com2torso * com2left.inverted(), rightLegAngles[JointsLeg::HIP_YAW_PITCH]);
    JointUtils::fillLegs(weightShiftAngles, leftLegAngles, rightLegAngles);
  }
  return weightShiftAngles;
}

JointsArray<float> Kick::computeLegAnglesFromFootPose(const JointsArray<float>& currentAngles,
                                                      const KinematicMatrix& nextLeft2right) const
{
  auto leftLegAngles = JointUtils::extractLeftLeg(currentAngles);
  auto rightLegAngles = JointUtils::extractRightLeg(currentAngles);

  // compute left and right foot pose relative to torso
  const KinematicMatrix right2torso = forwardKinematics().getRFoot(rightLegAngles);
  const KinematicMatrix left2torso = right2torso * nextLeft2right;

  // compute left and right leg angles
  leftLegAngles = inverseKinematics().getLLegAngles(left2torso);
  rightLegAngles =
      inverseKinematics().getFixedRLegAngles(right2torso, leftLegAngles[JointsLeg::HIP_YAW_PITCH]);

  JointsArray<float> nextAngles = currentAngles;
  JointUtils::fillLegs(nextAngles, leftLegAngles, rightLegAngles);
  return nextAngles;
}

void Kick::gyroFeedback(JointsArray<float>& outputAngles) const
{
  // add filtered gyroscope x and y values multiplied by gain to ankle roll and pitch, respectively
  outputAngles[Joints::R_ANKLE_ROLL] +=
      (leftKicking_ ? 1.f : -1.f) * gyroSidewaysBalanceFactor_() * filteredGyro_.x();
  outputAngles[Joints::R_ANKLE_PITCH] += gyroForwardBalanceFactor_() * filteredGyro_.y();
}

float Kick::parabolicStep(const float f)
{
  assert(f >= 0.f && f <= 1.f);
  if (f < 0.5f)
  {
    return 2.f * f * f;
  }
  return 4.f * f - 2.f * f * f - 1.f;
}

float Kick::parabolicPositiveStep(const float f)
{
  assert(f >= 0.f && f <= 1.f);
  return f * f;
}

float Kick::parabolicNegativeStep(const float f)
{
  assert(f >= 0.f && f <= 1.f);
  return 2 * f - f * f;
}
