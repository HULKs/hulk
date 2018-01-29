#include "Modules/Poses.h"

#include "Kick.hpp"


Kick::Kick(const ModuleManagerInterface& manager)
  : Module(manager, "Kick")
  , motionActivation_(*this)
  , motionRequest_(*this)
  , cycleInfo_(*this)
  , imuSensorData_(*this)
  , jointSensorData_(*this)
  , kickOutput_(*this)
  , leftMotionFile_(*this, "leftMotionFile")
  , rightMotionFile_(*this, "rightMotionFile")
  , leftMotion_(*cycleInfo_, *jointSensorData_)
  , rightMotion_(*cycleInfo_, *jointSensorData_)
  , kickPhaseHelper_(*this)
  , kickType_(KickType::STRAIGHT)
  , phase_(KickPhase::Phase::INACTIVE)
  , leftKicking_(false)
  , toReadyDuration_(*this, "toReadyDuration", [] {})
  , balanceDuration_(*this, "balanceDuration", [] {})
  , liftDuration_(*this, "liftDuration", [] {})
  , swingDuration_(*this, "swingDuration", [] {})
  , retractDuration_(*this, "retractDuration", [] {})
  , extendAndCenterDuration_(*this, "extendAndCenterDuration", [] {})
  , waitDuration_(*this, "waitDuration", [] {})
  , catchFallenDuration_(*this, "catchFallenDuration", [] {})
  , toReady_(*this, kickPhaseHelper_, toReadyDuration_())
  , balance_(*this, kickPhaseHelper_, balanceDuration_())
  , lift_(*this, kickPhaseHelper_, liftDuration_())
  , swing_(*this, kickPhaseHelper_, swingDuration_())
  , retract_(*this, kickPhaseHelper_, retractDuration_())
  , extendAndCenter_(*this, kickPhaseHelper_, extendAndCenterDuration_())
  , wait_(*this, kickPhaseHelper_, waitDuration_())
  , catchFallen_(*this, kickPhaseHelper_, catchFallenDuration_())
  , lowPassAlpha_(*this, "lowPassAlphaGyro", [] {})
  , gainX_(*this, "gainX", [] {})
  , gainY_(*this, "gainY", [] {})
{
  std::string motionFileRoot = robotInterface().getFileRoot() + "motions/";

  leftMotion_.loadFromFile(motionFileRoot + leftMotionFile_());
  rightMotion_.loadFromFile(motionFileRoot + rightMotionFile_());
}

void Kick::cycle()
{
  /// if a kick is requested, initial values are set and a kick begins
  const bool incomingKickRequest = motionActivation_->activations[static_cast<unsigned int>(MotionRequest::BodyMotion::KICK)] == 1 &&
                                   motionRequest_->bodyMotion == MotionRequest::BodyMotion::KICK;
  if (incomingKickRequest && phase_ == KickPhase::Phase::INACTIVE)
  {
    handleKickRequest();
  }

  /// the kick is aborted if the nao is detected to be fallen
  catchFallen();

  MotionFilePlayer::JointValues values;
  values.angles = Poses::getPose(Poses::READY);
  values.stiffnesses = std::vector<float>(JOINTS::JOINTS_MAX, 1.f);
  /// compute kick angles for the current phase and check if the next phase should commence
  switch (phase_)
  {
    case KickPhase::Phase::TO_READY:
    {
      toReady_.getAngles(values.angles, TIME_STEP);
      applyAnkleController(values.angles);
      if (toReady_.finished())
      {
        balance_.reset(Poses::getPose(Poses::READY));
        phase_ = KickPhase::Phase::BALANCE;
      }
      break;
    }
    case KickPhase::Phase::BALANCE:
    {
      balance_.getAngles(values.angles, TIME_STEP);
      applyAnkleController(values.angles);
      if (balance_.finished())
      {
        lift_.reset(values.angles);
        phase_ = KickPhase::Phase::LIFT;
      }
      break;
    }
    case KickPhase::Phase::LIFT:
    {
      lift_.getAngles(values.angles, TIME_STEP);
      applyAnkleController(values.angles);
      if (lift_.finished())
      {
        swing_.reset(values.angles);
        phase_ = KickPhase::Phase::SWING;
      }
      break;
    }
    case KickPhase::Phase::SWING:
    {
      swing_.getAngles(values.angles, TIME_STEP);
      applyAnkleController(values.angles);
      if (swing_.finished())
      {
        retract_.reset(values.angles);
        phase_ = KickPhase::Phase::RETRACT;
      }
      break;
    }
    case KickPhase::Phase::RETRACT:
    {
      retract_.getAngles(values.angles, TIME_STEP);
      applyAnkleController(values.angles);
      if (retract_.finished())
      {
        extendAndCenter_.reset();
        phase_ = KickPhase::Phase::EXTEND_AND_CENTER;
      }
      break;
    }
    case KickPhase::Phase::EXTEND_AND_CENTER:
    {
      extendAndCenter_.getAngles(values.angles, TIME_STEP);
      applyAnkleController(values.angles);
      if (extendAndCenter_.finished())
      {
        wait_.reset(values.angles);
        phase_ = KickPhase::Phase::WAIT;
      }
      break;
    }
    case KickPhase::Phase::WAIT:
    {
      wait_.getAngles(values.angles, TIME_STEP);
      if (wait_.finished())
      {
        phase_ = KickPhase::Phase::INACTIVE;
      }
      break;
    }
    case KickPhase::Phase::CATCH_FALLEN:
    {
      catchFallen_.getAngles(values.angles, TIME_STEP);
      if (catchFallen_.finished())
      {
        phase_ = KickPhase::Phase::INACTIVE;
      }
      break;
    }
    case KickPhase::Phase::MOTION_FILE:
    {
      if (leftMotion_.isPlaying())
      {
        values = leftMotion_.cycle();
      }
      else if (rightMotion_.isPlaying())
      {
        values = rightMotion_.cycle();
      }
      else
      {
        values.angles = Poses::getPose(Poses::READY);
        phase_ = KickPhase::Phase::INACTIVE;
      }
      break;
    }
    default:
    {
      values.angles = Poses::getPose(Poses::READY);
      kickOutput_->stiffnesses = std::vector<float>(JOINTS::JOINTS_MAX, 0.7f);
    }
  }

  /// set the output appropriately
  if (phase_ != KickPhase::Phase::INACTIVE)
  {
    /// output kick angles with all stiffnesses set to 1 if kick is active
    kickOutput_->angles = values.angles;
    kickOutput_->stiffnesses = values.stiffnesses;
    kickOutput_->safeExit = false;

    /// mirror kick output if rightkicking
    if (!leftKicking_ && phase_ != KickPhase::Phase::MOTION_FILE)
    {
      kickOutput_->mirrorAngles();
    }
  }
  else
  {
    /// output ready pose with all stiffnesses set to 0.7 otherwise
    kickOutput_->angles = Poses::getPose(Poses::READY);
    kickOutput_->stiffnesses = std::vector<float>(JOINTS::JOINTS_MAX, 0.7f);
    kickOutput_->safeExit = true;
  }
}

void Kick::handleKickRequest()
{
  Vector2f ballSource = motionRequest_->kickData.ballSource;
  Vector2f ballDestination = motionRequest_->kickData.ballDestination;
  kickType_ = motionRequest_->kickData.kickType;
  leftKicking_ = ballSource[1] > 0;
  angleAccumulator_ = imuSensorData_->angle;
  gyroAccumulator_ = imuSensorData_->gyroscope;
  std::vector<float> angles = jointSensorData_->getBodyAngles();

  /// handle kick type
  switch (kickType_)
  {
    case KickType::OLD:
    {
      if (leftKicking_)
      {
        leftMotion_.play();
      }
      else
      {
        rightMotion_.play();
      }
      phase_ = KickPhase::Phase::MOTION_FILE;
      break;
    }
    case KickType::STRAIGHT:
    {
      kickPhaseHelper_.resetStraightKick(leftKicking_, ballSource, ballDestination, angles);
      toReady_.reset(angles);
      phase_ = KickPhase::Phase::TO_READY;
      break;
    }
  }
}

void Kick::catchFallen()
{
  angleAccumulator_ = imuSensorData_->angle * lowPassAlpha_() + angleAccumulator_ * (1 - lowPassAlpha_());
  const bool fallen = std::abs(angleAccumulator_.x()) > 0.45f || std::abs(angleAccumulator_.y()) > 0.35f;
  const bool notYetCaught = phase_ != KickPhase::Phase::CATCH_FALLEN && phase_ != KickPhase::Phase::INACTIVE;
  if (fallen && notYetCaught)
  {
    catchFallen_.reset(jointSensorData_->getBodyAngles());
    phase_ = KickPhase::Phase::CATCH_FALLEN;
  }
}

void Kick::applyAnkleController(std::vector<float>& angles)
{
  Vector2f ankleCorrection;
  gyroAccumulator_ = imuSensorData_->gyroscope * lowPassAlpha_() + gyroAccumulator_ * (1 - lowPassAlpha_());
  ankleCorrection.x() = gyroAccumulator_.x() * gainX_();
  ankleCorrection.y() = gyroAccumulator_.y() * gainY_();
  if (leftKicking_)
  {
    angles[JOINTS::R_ANKLE_ROLL] += ankleCorrection.x();
    angles[JOINTS::R_ANKLE_PITCH] += ankleCorrection.y();
  }
  else
  {
    angles[JOINTS::L_ANKLE_ROLL] += ankleCorrection.x();
    angles[JOINTS::L_ANKLE_PITCH] += ankleCorrection.y();
  }
  kickPhaseHelper_.setPreviousAngles(angles);
}
