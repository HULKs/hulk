#include "Modules/NaoProvider.h"
#include "Modules/Poses.h"
#include "Tools/Kinematics/Com.h"
#include "Tools/Kinematics/ForwardKinematics.h"
#include "Tools/Kinematics/InverseKinematics.h"
#include "Tools/Math/Angle.hpp"

#include "Kick.hpp"


Kick::Kick(const ModuleManagerInterface& manager)
  : Module(manager)
  , motionActivation_(*this)
  , motionRequest_(*this)
  , cycleInfo_(*this)
  , imuSensorData_(*this)
  , jointSensorData_(*this)
  , kickOutput_(*this)
  , leftKicking_(true)
  , torsoOffsetLeft_(*this, "torsoOffsetLeft", [] {})
  , torsoOffsetRight_(*this, "torsoOffsetRight", [] {})
  , forwardKickParameters_(*this, "forwardKickParameters",
                           [this] {
                             forwardKickParameters_().yawLeft2right *= TO_RAD;
                             forwardKickParameters_().shoulderRoll *= TO_RAD;
                             forwardKickParameters_().shoulderPitchAdjustment *= TO_RAD;
                             forwardKickParameters_().ankleRoll *= TO_RAD;
                             forwardKickParameters_().anklePitch *= TO_RAD;
                           })
  , sideKickParameters_(*this, "sideKickParameters",
                        [this] {
                          sideKickParameters_().yawLeft2right *= TO_RAD;
                          sideKickParameters_().shoulderRoll *= TO_RAD;
                          sideKickParameters_().shoulderPitchAdjustment *= TO_RAD;
                          sideKickParameters_().ankleRoll *= TO_RAD;
                          sideKickParameters_().anklePitch *= TO_RAD;
                        })
  , currentInterpolatorID_(interpolators_.size())
  , gyroLowPassRatio_(*this, "gyroLowPassRatio", [] {})
  , gyroForwardBalanceFactor_(*this, "gyroForwardBalanceFactor", [] {})
  , gyroSidewaysBalanceFactor_(*this, "gyroSidewaysBalanceFactor", [] {})
  , filteredGyro_(Vector2f::Zero())
{
  forwardKickParameters_().yawLeft2right *= TO_RAD;
  forwardKickParameters_().shoulderRoll *= TO_RAD;
  forwardKickParameters_().shoulderPitchAdjustment *= TO_RAD;
  forwardKickParameters_().ankleRoll *= TO_RAD;
  forwardKickParameters_().anklePitch *= TO_RAD;
  sideKickParameters_().yawLeft2right *= TO_RAD;
  sideKickParameters_().shoulderRoll *= TO_RAD;
  sideKickParameters_().shoulderPitchAdjustment *= TO_RAD;
  sideKickParameters_().ankleRoll *= TO_RAD;
  sideKickParameters_().anklePitch *= TO_RAD;
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
      motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::KICK)] ==
          1 &&
      motionRequest_->bodyMotion == MotionRequest::BodyMotion::KICK;
  if (currentInterpolatorID_ == interpolators_.size() && incomingKickRequest)
  {
    // select kick parameters based on requested kick type
    KickParameters kickParameters;
    switch (motionRequest_->kickData.kickType)
    {
      case KickType::FORWARD:
      {
        kickParameters = forwardKickParameters_();
        break;
      }
      case KickType::SIDE:
      {
        kickParameters = sideKickParameters_();
        break;
      }
      default:
      {
        kickParameters = forwardKickParameters_();
        break;
      }
    }
    // check whether left or right foot is to be used
    leftKicking_ = motionRequest_->kickData.ballSource.y() > 0;
    // select appropriate torso offset
    const Vector3f torsoOffset = leftKicking_ ? torsoOffsetLeft_() : torsoOffsetRight_();
    // reset interpolators
    resetInterpolators(kickParameters, torsoOffset);
    // initialize kick
    currentInterpolatorID_ = 0;
  }

  // check whether kick if active
  if (currentInterpolatorID_ < interpolators_.size())
  {
    // do not move this check unless you want a segmentation fault
    if (interpolators_[currentInterpolatorID_]->finished())
    {
      // advance kick phase
      currentInterpolatorID_++;
    }
  }

  // check whether kick if active
  if (currentInterpolatorID_ < interpolators_.size())
  {
    // convert seconds to milliseconds to get time step
    const float timeStep = cycleInfo_->cycleTime * 1000;
    // get output angles from current interpolator
    std::vector<float> outputAngles = interpolators_[currentInterpolatorID_]->step(timeStep);
    // apply gyroscope feedback
    gyroFeedback(outputAngles);
    kickOutput_->angles = outputAngles;
    kickOutput_->stiffnesses = std::vector<float>(JOINTS::JOINTS_MAX, 1.f);
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
    kickOutput_->angles = Poses::getPose(Poses::READY);
    kickOutput_->stiffnesses = std::vector<float>(JOINTS::JOINTS_MAX, 0.7f);
    kickOutput_->safeExit = true;
  }
}

void Kick::resetInterpolators(const KickParameters& kickParameters, const Vector3f& torsoOffset)
{
  /*
   * wait before start
   */
  const std::vector<float> anglesAtKickRequest = jointSensorData_->getBodyAngles();
  const std::vector<float> readyPoseAngles = Poses::getPose(Poses::READY);
  waitBeforeStartInterpolator_.reset(anglesAtKickRequest, readyPoseAngles,
                                     kickParameters.waitBeforeStartDuration);

  /*
   * weight shift
   */
  const Vector3f weightShiftCom = kickParameters.weightShiftCom + torsoOffset;
  std::vector<float> weightShiftAngles(JOINTS::JOINTS_MAX);
  computeWeightShiftAnglesFromReferenceCom(readyPoseAngles, weightShiftCom, weightShiftAngles);
  weightShiftAngles[JOINTS::L_SHOULDER_ROLL] = kickParameters.shoulderRoll;
  weightShiftAngles[JOINTS::R_SHOULDER_ROLL] = -kickParameters.shoulderRoll;
  weightShiftInterpolator_.reset(readyPoseAngles, weightShiftAngles,
                                 kickParameters.weightShiftDuration);

  /*
   * lift foot
   */
  const float yawLeft2right = kickParameters.yawLeft2right;
  const KinematicMatrix liftFootPose = KinematicMatrix(AngleAxisf(yawLeft2right, Vector3f::UnitZ()),
                                                       kickParameters.liftFootPosition);
  std::vector<float> liftFootAngles(JOINTS::JOINTS_MAX);
  computeLegAnglesFromFootPose(weightShiftAngles, liftFootPose, liftFootAngles);
  liftFootAngles[JOINTS::L_SHOULDER_PITCH] -= kickParameters.shoulderPitchAdjustment;
  liftFootAngles[JOINTS::R_SHOULDER_PITCH] += kickParameters.shoulderPitchAdjustment;
  liftFootAngles[JOINTS::L_ANKLE_ROLL] = kickParameters.ankleRoll;
  liftFootInterpolator_.reset(weightShiftAngles, liftFootAngles, kickParameters.liftFootDuration);

  /*
   * swing foot
   */
  const KinematicMatrix swingFootPose = KinematicMatrix(
      AngleAxisf(yawLeft2right, Vector3f::UnitZ()), kickParameters.swingFootPosition);
  std::vector<float> swingFootAngles(JOINTS::JOINTS_MAX);
  computeLegAnglesFromFootPose(liftFootAngles, swingFootPose, swingFootAngles);
  swingFootAngles[JOINTS::L_SHOULDER_PITCH] += kickParameters.shoulderPitchAdjustment;
  swingFootAngles[JOINTS::R_SHOULDER_PITCH] -= kickParameters.shoulderPitchAdjustment;
  swingFootAngles[JOINTS::L_ANKLE_PITCH] += kickParameters.anklePitch;
  swingFootAngles[JOINTS::L_ANKLE_ROLL] = kickParameters.ankleRoll;
  swingFootInterpolator_.reset(liftFootAngles, swingFootAngles, kickParameters.swingFootDuration);

  /*
   * kick ball
   */
  const KinematicMatrix kickBallPose = KinematicMatrix(AngleAxisf(yawLeft2right, Vector3f::UnitZ()),
                                                       kickParameters.kickBallPosition);
  std::vector<float> kickBallAngles(JOINTS::JOINTS_MAX);
  computeLegAnglesFromFootPose(swingFootAngles, kickBallPose, kickBallAngles);
  kickBallAngles[JOINTS::L_SHOULDER_PITCH] += kickParameters.shoulderPitchAdjustment;
  kickBallAngles[JOINTS::R_SHOULDER_PITCH] -= kickParameters.shoulderPitchAdjustment;
  kickBallAngles[JOINTS::L_ANKLE_ROLL] = kickParameters.ankleRoll;
  kickBallInterpolator_.reset(swingFootAngles, kickBallAngles, kickParameters.kickBallDuration);

  /*
   * pause
   */
  pauseInterpolator_.reset(kickBallAngles, kickBallAngles, kickParameters.pauseDuration);

  /*
   * retract foot
   */
  const KinematicMatrix retractFootPose = KinematicMatrix(
      AngleAxisf(yawLeft2right, Vector3f::UnitZ()), kickParameters.retractFootPosition);
  std::vector<float> retractFootAngles(JOINTS::JOINTS_MAX);
  computeLegAnglesFromFootPose(kickBallAngles, retractFootPose, retractFootAngles);
  retractFootAngles[JOINTS::L_SHOULDER_PITCH] -= kickParameters.shoulderPitchAdjustment;
  retractFootAngles[JOINTS::R_SHOULDER_PITCH] += kickParameters.shoulderPitchAdjustment;
  retractFootAngles[JOINTS::L_ANKLE_ROLL] = kickParameters.ankleRoll;
  retractFootInterpolator_.reset(kickBallAngles, retractFootAngles,
                                 kickParameters.retractFootDuration);

  /*
   * extend foot and center torso
   */
  extendFootAndCenterTorsoInterpolator_.reset(retractFootAngles, readyPoseAngles,
                                              kickParameters.extendFootAndCenterTorsoDuration);

  /*
   * wait before exit
   */
  waitBeforeExitInterpolator_.reset(readyPoseAngles, readyPoseAngles,
                                    kickParameters.waitBeforeExitDuration);
}

void Kick::computeWeightShiftAnglesFromReferenceCom(const std::vector<float>& currentAngles,
                                                    const Vector3f& weightShiftCom,
                                                    std::vector<float>& weightShiftAngles) const
{
  weightShiftAngles = currentAngles;
  // iteratively move the torso to achieve the desired CoM
  for (unsigned int i = 0; i < 5; i++)
  {
    std::vector<float> leftLegAngles(JOINTS_L_LEG::L_LEG_MAX);
    std::vector<float> rightLegAngles(JOINTS_R_LEG::R_LEG_MAX);
    separateAngles(leftLegAngles, rightLegAngles, weightShiftAngles);

    KinematicMatrix com2torso = Com::getCom(weightShiftAngles);
    const KinematicMatrix right2torso = ForwardKinematics::getRFoot(rightLegAngles);
    const KinematicMatrix com2right = right2torso.invert() * com2torso;
    const KinematicMatrix left2torso = ForwardKinematics::getLFoot(leftLegAngles);
    const KinematicMatrix com2left = left2torso.invert() * com2torso;

    const Vector3f comError = com2right.posV - weightShiftCom;

    com2torso.posV += comError;

    leftLegAngles = InverseKinematics::getLLegAngles(com2torso * com2left.invert());
    rightLegAngles = InverseKinematics::getFixedRLegAngles(
        com2torso * com2right.invert(), leftLegAngles[JOINTS_L_LEG::L_HIP_YAW_PITCH]);
    combineAngles(weightShiftAngles, currentAngles, leftLegAngles, rightLegAngles);
  }
}

void Kick::computeLegAnglesFromFootPose(const std::vector<float>& currentAngles,
                                        const KinematicMatrix& nextLeft2right,
                                        std::vector<float>& nextAngles) const
{
  std::vector<float> leftLegAngles(JOINTS_L_LEG::L_LEG_MAX);
  std::vector<float> rightLegAngles(JOINTS_R_LEG::R_LEG_MAX);
  separateAngles(leftLegAngles, rightLegAngles, currentAngles);

  // compute left and right foot pose relative to torso
  const KinematicMatrix right2torso = ForwardKinematics::getRFoot(rightLegAngles);
  const KinematicMatrix left2torso = right2torso * nextLeft2right;

  // compute left and right leg angles
  leftLegAngles = InverseKinematics::getLLegAngles(left2torso);
  rightLegAngles = InverseKinematics::getFixedRLegAngles(
      right2torso, leftLegAngles[JOINTS_L_LEG::L_HIP_YAW_PITCH]);

  combineAngles(nextAngles, currentAngles, leftLegAngles, rightLegAngles);
}

void Kick::separateAngles(std::vector<float>& left, std::vector<float>& right,
                          const std::vector<float>& body) const
{
  left.resize(JOINTS_L_LEG::L_LEG_MAX);
  right.resize(JOINTS_R_LEG::R_LEG_MAX);
  for (unsigned int i = 0; i < JOINTS_L_LEG::L_LEG_MAX; i++)
  {
    left[i] = body[JOINTS::L_HIP_YAW_PITCH + i];
    right[i] = body[JOINTS::R_HIP_YAW_PITCH + i];
  }
}

void Kick::combineAngles(std::vector<float>& result, const std::vector<float>& body,
                         const std::vector<float>& left, const std::vector<float>& right) const
{
  result = body;
  for (unsigned int i = 0; i < JOINTS_L_LEG::L_LEG_MAX; i++)
  {
    result[JOINTS::L_HIP_YAW_PITCH + i] = left[i];
    result[JOINTS::R_HIP_YAW_PITCH + i] = right[i];
  }
}

void Kick::gyroFeedback(std::vector<float>& outputAngles) const
{
  // add filtered gyroscope x and y values multiplied by gain to ankle roll and pitch, respectively
  outputAngles[JOINTS::R_ANKLE_ROLL] +=
      (leftKicking_ ? 1 : -1) * gyroSidewaysBalanceFactor_() * filteredGyro_.x();
  outputAngles[JOINTS::R_ANKLE_PITCH] += gyroForwardBalanceFactor_() * filteredGyro_.y();
}
