#include "UNSW2014Generator.hpp"

#include "Modules/NaoProvider.h"
#include "Modules/Poses.h"
#include "Tools/Kinematics/Com.h"
#include "Tools/Kinematics/ForwardKinematics.h"
#include "Tools/Kinematics/InverseKinematics.h"
#include "Tools/Math/Angle.hpp"
#include "print.h"
#include <cmath>

static const float mmPerM = 1000.f;

UNSW2014Generator::UNSW2014Generator(const ModuleManagerInterface& manager)
  : Module(manager)
  , maxSpeed_(*this, "maxSpeed", [this] { maxSpeed_().orientation *= TO_RAD; })
  , maxSpeedBackwards_(*this, "maxSpeedBackwards", [] {})
  , maxAcceleration_(*this, "maxAcceleration", [] {})
  , maxDeceleration_(*this, "maxDeceleration", [] {})
  , slowMaxSpeed_(*this, "slowMaxSpeed", [this] { slowMaxSpeed_().orientation *= TO_RAD; })
  , slowMaxSpeedBackwards_(*this, "slowMaxSpeedBackwards", [] {})
  , slowMaxAcceleration_(*this, "slowMaxAcceleration", [] {})
  , walkVolumeTranslationExponent_(*this, "walkVolumeTranslationExponent", [] {})
  , walkVolumeRotationExponent_(*this, "walkVolumeRotationExponent", [] {})
  , baseWalkPeriod_(*this, "baseWalkPeriod", [] {})
  , sidewaysWalkPeriodIncreaseFactor_(*this, "sidewaysWalkPeriodIncreaseFactor", [] {})
  , walkHipHeight_(*this, "walkHipHeight", [] {})
  , baseFootLift_(*this, "baseFootLift", [] {})
  , footLiftIncreaseFactor_(*this, "footLiftIncreaseFactor", [] {})
  , footLiftFirstStepFactor_(*this, "footLiftFirstStepFactor", [] {})
  , supportSwitchPhaseRange_(*this, "supportSwitchPhaseRange", [] {})
  , maxWeightShiftMisses_(*this, "maxWeightShiftMisses", [] {})
  , emergencyStepSize_(*this, "emergencyStepSize", [] {})
  , minSlowWeightShiftRatio_(*this, "minSlowWeightShiftRatio", [] {})
  , maxSlowWeightShifts_(*this, "maxSlowWeightShifts", [] {})
  , slowWaitShiftStandDelay_(*this, "slowWaitShiftStandDelay", [] {})
  , insideTurnRatio_(*this, "insideTurnRatio", [] {})
  , torsoOffset_(*this, "torsoOffset", [] {})
  , speedScale_(*this, "speedScale", [] {})
  , odometryScale_(*this, "odometryScale", [] {})
  , walkLegStiffness_(*this, "walkLegStiffness", [] {})
  , standLegStiffness_(*this, "standLegStiffness", [] {})
  , armStiffness_(*this, "armStiffness", [] {})
  , armShoulderRoll_(*this, "armShoulderRoll", [this] { armShoulderRoll_() *= TO_RAD; })
  , armShoulderRollIncreaseFactor_(*this, "armShoulderRollIncreaseFactor", [] {})
  , armShoulderPitchFactor_(*this, "armShoulderPitchFactor", [] {})
  , gyroLowPassRatio_(*this, "gyroLowPassRatio", [] {})
  , accelerometerLowPassRatio_(*this, "accelerometerLowPassRatio", [] {})
  , gyroForwardBalanceFactor_(*this, "gyroForwardBalanceFactor", [] {})
  , gyroBackwardBalanceFactor_(*this, "gyroBackwardBalanceFactor", [] {})
  , gyroSidewaysBalanceFactor_(*this, "gyroSidewaysBalanceFactor", [] {})
  , targetModeSpeedFactor_(*this, "targetModeSpeedFactor", [] {})
  , enableReturnOffset_(*this, "enableReturnOffset", [] {})
  , enableTorsoCompensation_(*this, "enableTorsoCompensation", [] {})
  , headComGain_(*this, "headComGain", [] {})
  , armComGain_(*this, "armComGain", [] {})
  , speedCompensationGain_(*this, "speedCompensationGain", [] {})
  , accelerationCompensationGain_(*this, "accelerationCompensationGain", [] {})
  , enableGyroBalanceInStand_(*this, "enableGyroBalanceInStand", [] {})
  , enableCollisionReaction_(*this, "enableCollisionReaction", [] {})
  , triggerDebugCollision_(*this, "triggerDebugCollision", [] {})
  , armLiftDuration_(*this, "armLiftDuration", [] {})
  , armPullTightDuration_(*this, "armPullTightDuration", [] {})
  , bodyPose_(*this)
  , cycleInfo_(*this)
  , imuSensorData_(*this)
  , jointSensorData_(*this)
  , robotKinematics_(*this)
  , collisionDetectorData_(*this)
  , motionRequest_(*this)
  , walkGenerator_(*this)
  , filteredAccelerometer_(Vector3f::Zero())
  , lastProjectedTorso2Support_(Vector2f::Zero())
  , lastProjectedTorsoShift_(Vector2f::Zero())
  , lastStepwiseTorsoCompensation_(0)
  , armInterpolator1_()
  , armInterpolator2_()
  , armState_(WalkGenerator::ArmState::NORMAL)
{
  // initial unit conversions to rad
  maxSpeed_().orientation *= TO_RAD;
  slowMaxSpeed_().orientation *= TO_RAD;
  armShoulderRoll_() *= TO_RAD;
  // set to false just for safety reasons. This way one has to set this parameter at least once via
  // config
  triggerDebugCollision_() = false;

  nextArmAngles_.resize(JOINTS_L_ARM::L_ARM_MAX + JOINTS_R_ARM::R_ARM_MAX);
  readyArmAngles_.resize(JOINTS_L_ARM::L_ARM_MAX + JOINTS_R_ARM::R_ARM_MAX);
  armLiftAngles_.resize(JOINTS_L_ARM::L_ARM_MAX + JOINTS_R_ARM::R_ARM_MAX);
  armPullTightAngles_.resize(JOINTS_L_ARM::L_ARM_MAX + JOINTS_R_ARM::R_ARM_MAX);

  // get relevant angles from poses
  for (unsigned int i = 0; i < JOINTS_L_ARM::L_ARM_MAX; ++i)
  {
    readyArmAngles_[i] = Poses::getPose(Poses::READY)[JOINTS::L_SHOULDER_PITCH + i];
    armLiftAngles_[i] = Poses::getPose(Poses::ARMBACKSTAGE1)[JOINTS::L_SHOULDER_PITCH + i];
    armPullTightAngles_[i] = Poses::getPose(Poses::ARMBACKSTAGE2)[JOINTS::L_SHOULDER_PITCH + i];

    readyArmAngles_[JOINTS_L_ARM::L_ARM_MAX + i] =
        Poses::getPose(Poses::READY)[JOINTS::R_SHOULDER_PITCH + i];
    armLiftAngles_[JOINTS_L_ARM::L_ARM_MAX + i] =
        Poses::getPose(Poses::ARMBACKSTAGE1)[JOINTS::R_SHOULDER_PITCH + i];
    armPullTightAngles_[JOINTS_L_ARM::L_ARM_MAX + i] =
        Poses::getPose(Poses::ARMBACKSTAGE2)[JOINTS::R_SHOULDER_PITCH + i];
  }

  // Be safe and initialize with ready pose
  nextArmAngles_ = readyArmAngles_;
}

void UNSW2014Generator::cycle()
{
  // filter the sensor data that is used for feedback
  filterSensorData();
  // basically bind this module's function to the production
  walkGenerator_->resetGenerator = [this]() { resetGenerator(); };
  walkGenerator_->calcJoints =
      [this](const Pose& speed, const Pose& target, const Pose& walkPathGradient,
             WalkGenerator::WalkMode walkMode,
             const std::function<KinematicMatrix(float phase)>& getKickFootOffset) {
        calcJoints(speed, target, walkPathGradient, walkMode, getKickFootOffset);
      };
  walkGenerator_->maxSpeed = Pose(maxSpeed_().position.x() / speedScale_().position.x(), //
                                  maxSpeed_().position.y() / speedScale_().position.y(), //
                                  maxSpeed_().orientation / speedScale_().orientation);
}

void UNSW2014Generator::resetGenerator()
{
  walkGenerator_->stepDuration = 0.f;
  walkGenerator_->t = 0.f;
  walkState_ = WalkState::STANDING;
  forward_ = lastForward_ = 0.f;
  forwardL_ = forwardL0_ = 0.f;
  forwardR_ = forwardR0_ = 0.f;
  left_ = lastLeft_ = 0.f;
  leftL_ = leftR_ = 0;
  turnRL_ = turnRL0_ = 0;
  swingAngle_ = 0;
  switchPhase_ = 0.f;
  maxFootHeight_ = maxFootHeight0_ = 0.f;
  weightShiftStatus_ = WeightShiftStatus::WEIGHT_DID_NOT_SHIFT;
  filteredGyroX_ = filteredGyroY_ = 0;
  filteredAccelerometer_ = Vector3f::Zero();
  prevForwardL_ = prevForwardR_ = 0.f;
  prevLeftL_ = prevLeftR_ = 0;
  prevTurn_ = 0;
  weightShiftMisses_ = 0;
  slowWeightShifts_ = 0;
}

void UNSW2014Generator::calcJoints(
    const Pose& speed, const Pose& target, const Pose& walkPathGradient,
    WalkGenerator::WalkMode walkMode,
    const std::function<KinematicMatrix(float phase)>& getKickFootOffset)
{
  // 1. Read in new walk values (forward, left, turn, power) only at the start of a walk step
  // cycle, ie when t = 0
  if (walkGenerator_->t == 0)
  {
    initializeStepStatesFromRequest(speed, target, walkPathGradient, walkMode);
  }
  // 2. Update timer
  walkGenerator_->t += cycleInfo_->cycleTime;
  // 3. Determine Walk2014 Option
  if (walkState_ != WalkState::STANDING && forward_ == 0 && left_ == 0 && turn_ == 0)
  {
    walkState_ = WalkState::STOPPING;
  }
  else if (walkState_ != WalkState::WALKING && (forward_ != 0 || left_ != 0 || turn_ != 0))
  {
    walkState_ = WalkState::STARTING;
  }
  // 5. Determine walk variables throughout the walk step phase
  float foothL = 0.f;
  float foothR = 0.f;
  if (walkState_ == WalkState::STANDING)
  {
    walkGenerator_->stepDuration = walkGenerator_->t = 0.f;
    foothL = foothR = 0;
  }
  else
  {
    // 5.3 Calculate intra-walkphase forward, left and turn at time-step dt
    if (walkGenerator_->isLeftPhase)
    {
      calcFootOffsets(1.f, forwardL0_, forwardR0_, forwardL_, forwardR_, leftL_, leftR_, foothL,
                      foothR);
    }
    else
    {
      calcFootOffsets(-1.f, forwardR0_, forwardL0_, forwardR_, forwardL_, leftR_, leftL_, foothR,
                      foothL);
    }

    // 5.4 Special conditions when priming the walk
    if (walkState_ == WalkState::STARTING)
    {
      foothL *= footLiftFirstStepFactor_(); // reduce max lift due to short duration
      foothR *= footLiftFirstStepFactor_();
      forwardL_ = forwardR_ = 0; // don't move on starting
      leftR_ = leftL_ = 0;
      turnRL_ = 0;
      walkGenerator_->speed = Pose();
      if (left_ != 0.f) // make first real step in direction of movement
        walkGenerator_->isLeftPhase = left_ < 0;
    }
  }
  // 6. Changing Support Foot. Note isLeftPhase means left foot is swing foot.
  // t>0.75*T tries to avoid bounce, especially when side-stepping
  // lastZMPL*ZMPL<0.0 indicates that support foot has changed
  // t>3*T tries to get out of "stuck" situations
  const bool supportChangedInTime =
      walkGenerator_->t > supportSwitchPhaseRange_().min * walkGenerator_->stepDuration &&
      bodyPose_->supportChanged;
  const bool stepPhaseTookTooLong =
      walkGenerator_->t > supportSwitchPhaseRange_().max * walkGenerator_->stepDuration;
  // a step phase ends if the support foot changed (after MINIMUM half of the expected step
  // duration) or if the step took too long and we want to force the end
  bool supportChangedThisCycle = false;
  if (supportChangedInTime || stepPhaseTookTooLong)
  {
    supportChangedThisCycle = handleSupportPhaseEnd();
  }
  // 8. Odometry update for localization
  walkGenerator_->odometryOffset =
      calcMeasuredOdometryOffset(walkGenerator_->isLeftPhase, supportChangedThisCycle);
  // 9.1 Foot poses
  // Now assemble the kinemtic matrices for both feet from the calculationss above.
  // This also adds compensation and calibration.
  KinematicMatrix leftFoot2Torso = calcFoot2TorsoFromOffsets(1, turnRL_, leftL_, forwardL_, foothL);
  KinematicMatrix rightFoot2Torso =
      calcFoot2TorsoFromOffsets(-1, turnRL_, leftR_, forwardR_, foothR);
  // 9.2 Walk kicks
  if (getKickFootOffset)
  {
    // TODO: I think the order of multiplication is correct here. However, it's quite late right
    // now. If things are weird, check order. This sould be checked as soon as the first
    // in-walk-kick is implemented
    (walkGenerator_->isLeftPhase ? leftFoot2Torso : rightFoot2Torso) *=
        (getKickFootOffset(std::min(walkGenerator_->t / walkGenerator_->stepDuration, 1.f)));
  }
  // 9.3 Inverse kinematics
  walkGenerator_->angles = Poses::getPose(Poses::READY);
  calculateBodyAnglesFromFootPoses(leftFoot2Torso, rightFoot2Torso, walkGenerator_->isLeftPhase,
                                   walkGenerator_->angles);
  // 10. Set joint values and stiffness
  float legStiffness =
      walkState_ == WalkState::STANDING ? standLegStiffness_() : walkLegStiffness_();
  // set the default stiffness for all joints
  walkGenerator_->stiffnesses = std::vector<float>(JOINTS::JOINTS_MAX, legStiffness);
  // use a lower stiffness for the arms
  for (int i = 0; i < JOINTS_L_ARM::L_ARM_MAX; i++)
  {
    walkGenerator_->stiffnesses[JOINTS::L_SHOULDER_PITCH + i] = armStiffness_();
    walkGenerator_->stiffnesses[JOINTS::R_SHOULDER_PITCH + i] = armStiffness_();
  }
  // 10.1 Arms
  handleArms();
  // 7. Sagittal balance
  // adjust ankle tilt in proportion to filtered gyroY
  float balanceAdjustment =
      walkState_ == WalkState::STANDING && !enableGyroBalanceInStand_()
          ? 0.f
          : filteredGyroY_ *
                (filteredGyroY_ > 0 ? gyroForwardBalanceFactor_() : gyroBackwardBalanceFactor_());
  walkGenerator_
      ->angles[walkGenerator_->isLeftPhase ? JOINTS::R_ANKLE_PITCH : JOINTS::L_ANKLE_PITCH] +=
      balanceAdjustment;
  // Lateral balance
  if (walkState_ == WalkState::STANDING)
  {
    balanceAdjustment = filteredGyroX_ * gyroSidewaysBalanceFactor_();
    walkGenerator_->angles[JOINTS::L_ANKLE_ROLL] += balanceAdjustment;
    walkGenerator_->angles[JOINTS::R_ANKLE_ROLL] += balanceAdjustment;
  }
}

void UNSW2014Generator::handleArms()
{
  const bool collisionPredicted =
      triggerDebugCollision_() || collisionDetectorData_->collisionLeftRigid ||
      collisionDetectorData_->collisionRightRigid || collisionDetectorData_->duelRigid;

  // if the arms are still in normal mode and collisionPredicted is approaching we move them back
  if (enableCollisionReaction_() && collisionPredicted && walkGenerator_->t == 0 &&
      armState_ == WalkGenerator::ArmState::NORMAL && bodyPose_->footContact &&
      (motionRequest_->bodyMotion == MotionRequest::BodyMotion::WALK ||
       motionRequest_->bodyMotion == MotionRequest::BodyMotion::STAND))
  {
    // reset the arm interpolators to move the arms back
    armInterpolator1_.reset(getCurrentArmAngles(), armLiftAngles_, armLiftDuration_());
    armInterpolator2_.reset(armLiftAngles_, armPullTightAngles_, armPullTightDuration_());
    armState_ = WalkGenerator::ArmState::MOVING_BACK;
  }
  else if ((!enableCollisionReaction_() || !collisionPredicted || !bodyPose_->footContact ||
            (motionRequest_->bodyMotion != MotionRequest::BodyMotion::WALK &&
             motionRequest_->bodyMotion != MotionRequest::BodyMotion::STAND)) &&
           walkGenerator_->t == 0 && armState_ == WalkGenerator::ArmState::BACK)
  {
    // rest the arm interplator to move the arms to the front
    armInterpolator1_.reset(getCurrentArmAngles(), armLiftAngles_, armPullTightDuration_());
    armInterpolator2_.reset(armLiftAngles_, readyArmAngles_, armLiftDuration_());
    armState_ = WalkGenerator::ArmState::MOVING_FRONT;
  }

  if (armState_ == WalkGenerator::ArmState::MOVING_FRONT ||
      armState_ == WalkGenerator::ArmState::MOVING_BACK)
  {
    // if we are currently in a transition from front to back or vice versa, we simply continue with
    // that interpolation
    if (!armInterpolator1_.finished())
    {
      nextArmAngles_ = armInterpolator1_.step(cycleInfo_->cycleTime);
    }
    else if (!armInterpolator2_.finished())
    {
      nextArmAngles_ = armInterpolator2_.step(cycleInfo_->cycleTime);
    }
    else
    {
      assert(false);
    }
  }
  else if (armState_ == WalkGenerator::ArmState::NORMAL)
  {
    // "natural" arm swing while walking to counterbalance foot swing
    nextArmAngles_[JOINTS_L_ARM::L_SHOULDER_PITCH] =
        90 * TO_RAD - forwardL_ * armShoulderPitchFactor_();
    nextArmAngles_[JOINTS_L_ARM::L_ARM_MAX + JOINTS_R_ARM::R_SHOULDER_PITCH] =
        90 * TO_RAD - forwardR_ * armShoulderPitchFactor_();
    nextArmAngles_[JOINTS_L_ARM::L_SHOULDER_ROLL] =
        armShoulderRoll_() + std::abs(left_) * armShoulderRollIncreaseFactor_();
    nextArmAngles_[JOINTS_L_ARM::L_ARM_MAX + JOINTS_R_ARM::R_SHOULDER_ROLL] =
        -nextArmAngles_[JOINTS_L_ARM::L_SHOULDER_ROLL];
  }

  if (armInterpolator2_.finished() && (armState_ == WalkGenerator::ArmState::MOVING_FRONT ||
                                       armState_ == WalkGenerator::ArmState::MOVING_BACK))
  {
    // the current arm motion is finished thus we can advance the state
    armState_ = static_cast<WalkGenerator::ArmState>((static_cast<int>(armState_) + 1) % 4);
  }

  setArmAngles(nextArmAngles_);
  walkGenerator_->armState = armState_;
}

std::vector<float> UNSW2014Generator::getCurrentArmAngles() const
{
  std::vector<float> targetAngles;
  targetAngles.reserve(JOINTS_L_ARM::L_ARM_MAX + JOINTS_R_ARM::R_ARM_MAX);
  // fill the arm angles with the angles of the left arm
  targetAngles = jointSensorData_->getLArmAngles();
  // now also insert the right arm
  const auto rightArm = jointSensorData_->getRArmAngles();
  targetAngles.insert(targetAngles.end(), rightArm.begin(), rightArm.end());
  assert(targetAngles.size() == JOINTS_L_ARM::L_ARM_MAX + JOINTS_R_ARM::R_ARM_MAX);
  return targetAngles;
}

void UNSW2014Generator::setArmAngles(const std::vector<float>& armAngles)
{
  for (unsigned int i = 0; i < JOINTS_L_ARM::L_ARM_MAX; ++i)
  {
    walkGenerator_->angles[JOINTS::L_SHOULDER_PITCH + i] = armAngles[i];
    walkGenerator_->angles[JOINTS::R_SHOULDER_PITCH + i] = armAngles[JOINTS_L_ARM::L_ARM_MAX + i];
  }
}

void UNSW2014Generator::filterSensorData()
{
  filteredGyroY_ = gyroLowPassRatio_() * filteredGyroY_ +
                   (1.f - gyroLowPassRatio_()) * imuSensorData_->gyroscope.y();
  filteredGyroX_ = gyroLowPassRatio_() * filteredGyroX_ +
                   (1.f - gyroLowPassRatio_()) * imuSensorData_->gyroscope.x();

  filteredAccelerometer_ = accelerometerLowPassRatio_() * filteredAccelerometer_ +
                           (1.f - accelerometerLowPassRatio_()) * imuSensorData_->accelerometer;
}

void UNSW2014Generator::initializeStepStatesFromRequest(const Pose& speed, const Pose& target,
                                                        const Pose& walkPathGradient,
                                                        WalkGenerator::WalkMode walkMode)
{
  // 1. Read in new walk values (forward, left, turn, power) only at the start of a walk step
  // cycle, ie when t = 0
  Pose request = speed;
  Pose modifiedMaxSpeed = maxSpeed_();
  float modifiedMaxSpeedBackwards = maxSpeedBackwards_();
  // if we stop during step we will still have to lign up with the support foot.
  // Thus, only from returning to the stand pose we will move by this offset
  Pose returnOffset =
      enableReturnOffset_()
          ? Pose((walkGenerator_->isLeftPhase ? -forwardR0_ : -forwardL0_) /
                     speedScale_().position.x(),
                 (walkHipHeight_() + NaoProvider::link(LINKS::HIP_OFFSET_Z) / mmPerM -
                  NaoProvider::link(LINKS::FOOT_HEIGHT)) *
                     std::tan(swingAngle_) / speedScale_().position.y(),
                 (walkGenerator_->isLeftPhase ? -turnRL0_ : turnRL0_) / speedScale_().orientation)
          : Pose();

  if (cycleInfo_->getTimeDiff(timeWhenSlowWeightShiftsDetected_) <= slowWaitShiftStandDelay_())
  {
    request = Pose();
    walkMode = WalkGenerator::WalkMode::STEP_SIZE_MODE;
  }
  else if (weightShiftStatus_ == WeightShiftStatus::EMERGENCY_STEP)
  {
    request =
        Pose(0.f, walkGenerator_->isLeftPhase ? emergencyStepSize_() : -emergencyStepSize_(), 0.f);
    walkMode = WalkGenerator::WalkMode::STEP_SIZE_MODE;
    weightShiftStatus_ = WeightShiftStatus::WEIGHT_DID_SHIFT;
  }

  if (walkMode == WalkGenerator::WalkMode::TARGET_MODE)
  {
    if (!(speed.orientation > 0.f && speed.position.x() > 0.f && speed.position.y() > 0.f))
    {
      walkMode = WalkGenerator::WalkMode::VELOCITY_MODE;
      request = Pose(target.position * targetModeSpeedFactor_(),
                     target.orientation * targetModeSpeedFactor_());
    }
    else
    {
      modifiedMaxSpeed =
          Pose(std::min(speed.position.x() * speedScale_().position.x(), maxSpeed_().position.x()),
               std::min(speed.position.y() * speedScale_().position.y(), maxSpeed_().position.y()),
               std::min(speed.orientation * speedScale_().orientation,
                        static_cast<float>(maxSpeed_().orientation)));
      modifiedMaxSpeedBackwards =
          std::min(speed.position.x() * speedScale_().position.x(), maxSpeedBackwards_());
      // Remove the offset that will be covered just by returning the swing leg
      forward_ = (target.position.x() - returnOffset.position.x()) * speedScale_().position.x();
      left_ = (target.position.y() - returnOffset.position.y()) * speedScale_().position.y();
      turn_ = (target.orientation - returnOffset.orientation) * speedScale_().orientation;
      walkGenerator_->stepDuration =
          (baseWalkPeriod_() + sidewaysWalkPeriodIncreaseFactor_() * std::abs(left_));
      walkGenerator_->speed =
          Pose(forward_ / walkGenerator_->stepDuration, left_ / walkGenerator_->stepDuration,
               turn_ / walkGenerator_->stepDuration);
      // ellipsoidClampWalk returns true if clamped. Thus, if the target can not be reached in
      // this step, the following condition is true
      if (ellipsoidClampWalk(modifiedMaxSpeed, modifiedMaxSpeedBackwards,
                             walkGenerator_->speed.position.x(), walkGenerator_->speed.position.y(),
                             walkGenerator_->speed.orientation))
      {
        // if the target can not be reached in this step, we can simply use the speed mode and
        // ignore the target
        walkMode = WalkGenerator::WalkMode::VELOCITY_MODE;

        const float maxTargetDistanceVelocity = target.position.norm() * targetModeSpeedFactor_();
        const float requestedVelocity = walkPathGradient.position.norm();
        const Vector2f walkDirection = walkPathGradient.position.normalized();

        request = Pose(requestedVelocity > maxTargetDistanceVelocity
                           ? walkDirection * maxTargetDistanceVelocity
                           : walkPathGradient.position,
                       walkPathGradient.orientation * targetModeSpeedFactor_());
      }
      else
      {
        // Consider in the speed that half of the step is returning to origin
        walkGenerator_->speed.orientation = 0.5f * walkGenerator_->speed.orientation +
                                            returnOffset.orientation / walkGenerator_->stepDuration;
        walkGenerator_->speed.position = 0.5f * walkGenerator_->speed.position +
                                         returnOffset.position / walkGenerator_->stepDuration;
      }
    }
  }

  if (walkMode == WalkGenerator::WalkMode::VELOCITY_MODE)
  {
    forward_ = request.position.x() * speedScale_().position.x();
    left_ = request.position.y() * speedScale_().position.y();
    turn_ = request.orientation * speedScale_().orientation;
    // Scale back values to try to ensure stability.
    ellipsoidClampWalk(modifiedMaxSpeed, modifiedMaxSpeedBackwards, forward_, left_, turn_);
    // If switching direction, first stop if new speed is not reachable through acceleration
    if (lastForward_ * forward_ < 0.f && std::abs(lastForward_) > std::abs(maxAcceleration_().x()))
    {
      forward_ = 0.f;
    }
    // Limit acceleration and deceleration of forward movement
    if (lastForward_ > 0.f || (lastForward_ == 0.f && forward_ > 0.f))
    {
      forward_ = lastForward_ + Range<float>::clipToGivenRange(forward_ - lastForward_,
                                                               -maxDeceleration_().x(),
                                                               maxAcceleration_().x());
    }
    else
    {
      forward_ = lastForward_ + Range<float>::clipToGivenRange(forward_ - lastForward_,
                                                               -maxAcceleration_().x(),
                                                               maxDeceleration_().x());
    }
    // If switching direction, first stop if new speed is not reachable through acceleration
    if (lastLeft_ * left_ < 0.f && std::abs(lastLeft_) > std::abs(maxAcceleration_().y()))
    {
      left_ = 0.f;
    }
    // Limit acceleration and deceleration of sideways movement
    if (lastLeft_ > 0.f || (lastLeft_ == 0.f && left_ > 0.f))
    {
      left_ = lastLeft_ + Range<float>::clipToGivenRange(left_ - lastLeft_, -maxDeceleration_().y(),
                                                         maxAcceleration_().y());
    }
    else
    {
      left_ = lastLeft_ + Range<float>::clipToGivenRange(left_ - lastLeft_, -maxAcceleration_().y(),
                                                         maxDeceleration_().y());
    }
    walkGenerator_->stepDuration =
        (baseWalkPeriod_() + sidewaysWalkPeriodIncreaseFactor_() * std::abs(left_));
    // Consider in the speed that half of the step is returning to origin
    walkGenerator_->speed = Pose(0.5f * forward_ / speedScale_().position.x() +
                                     returnOffset.position.x() / walkGenerator_->stepDuration,
                                 0.5f * left_ / speedScale_().position.y() +
                                     returnOffset.position.y() / walkGenerator_->stepDuration,
                                 0.5f * turn_ / speedScale_().orientation +
                                     returnOffset.orientation / walkGenerator_->stepDuration);
  }
  else if (walkMode == WalkGenerator::WalkMode::STEP_SIZE_MODE)
  {
    forward_ = request.position.x();
    left_ = request.position.y();
    turn_ = request.orientation;
    walkGenerator_->stepDuration =
        (baseWalkPeriod_() + sidewaysWalkPeriodIncreaseFactor_() * std::abs(left_));

    // Consider in the speed that half of the step is returning to origin
    walkGenerator_->speed =
        Pose((0.5f * forward_ + returnOffset.position.x()) / walkGenerator_->stepDuration,
             (0.5f * left_ + returnOffset.position.y()) / walkGenerator_->stepDuration,
             (0.5f * turn_ + returnOffset.orientation) / walkGenerator_->stepDuration);
  }

  if (walkMode == WalkGenerator::WalkMode::VELOCITY_MODE)
  {
    // 1.6 Walk Calibration
    // The definition of forward, left and turn is the actual distance/angle traveled in one
    // second. It is scaled down to the duration of a single step.

    // store the velocities in forward and left direction for the next cycle.
    lastForward_ = forward_;
    lastLeft_ = left_;
    forward_ *= walkGenerator_->stepDuration;
    left_ *= walkGenerator_->stepDuration;
    turn_ *= walkGenerator_->stepDuration;
  }
  else
  {
    // in case we are not in velocity mode yet but we want to use it the next cycle we need to
    // know the last speed. At this point forward and left will contain step sizes. Thus we need
    // to convert it to a speed
    lastForward_ = forward_ / walkGenerator_->stepDuration;
    lastLeft_ = left_ / walkGenerator_->stepDuration;
  }

  // 5.1 Calculate the height to lift each swing foot
  maxFootHeight_ = baseFootLift_() + std::abs(forward_) * footLiftIncreaseFactor_().x() +
                   std::abs(left_) * footLiftIncreaseFactor_().y();
  debug().update("inTargetMode", walkMode == WalkGenerator::WalkMode::TARGET_MODE);
}

bool UNSW2014Generator::handleSupportPhaseEnd()
{
  bool supportChangedThisCycle = false;
  lastStepwiseTorsoCompensation_ = getStepwiseTorsoCompensation();
  switchPhase_ = walkGenerator_->t;
  maxFootHeight0_ = maxFootHeight_;
  weightShiftStatus_ = walkGenerator_->isLeftPhase != (bodyPose_->supportSide < 0)
                           ? WeightShiftStatus::WEIGHT_DID_SHIFT
                           : WeightShiftStatus::WEIGHT_DID_NOT_SHIFT;
  walkGenerator_->isLeftPhase = bodyPose_->supportSide < 0;

  if (weightShiftStatus_ == WeightShiftStatus::WEIGHT_DID_NOT_SHIFT)
  {
    lastForward_ = lastLeft_ = 0;
    if (++weightShiftMisses_ > maxWeightShiftMisses_())
    {
      Log(LogLevel::INFO) << "Walk2014Generator: Too many weight shift misses";
      weightShiftStatus_ = WeightShiftStatus::EMERGENCY_STEP;
      walkGenerator_->isLeftPhase = !walkGenerator_->isLeftPhase;
      weightShiftMisses_ = 0;
    }
  }
  else
  {
    supportChangedThisCycle = true;
    if (switchPhase_ > minSlowWeightShiftRatio_() * walkGenerator_->stepDuration)
    {
      if (++slowWeightShifts_ > maxSlowWeightShifts_())
      {
        Log(LogLevel::INFO) << "Walk2014Generator: Too many slow weight shifts";
        timeWhenSlowWeightShiftsDetected_ = cycleInfo_->startTime;
      }
    }
    else
    {
      slowWeightShifts_ = 0;
    }
  }

  if (walkState_ != WalkState::STANDING)
  {
    // 6.1 Recover previous "left" swing angle
    // store the end position of the swinging foot for the next step
    swingAngle_ = walkGenerator_->isLeftPhase ? leftL_ : leftR_;

    // 6.2 Decide on timing of next walk step phase
    // If we are not standing and not walking we are stopping or starting
    if (walkState_ != WalkState::WALKING)
    {
      // the stopping and and starting state can only be held for one step
      // at support change time we always proceed to the next WalkState (with overflow)
      // starting -> walking, stopping -> standing
      walkState_ = static_cast<WalkState>((static_cast<int>(walkState_) + 1) & 3);
    }

    // 6.3 reset step phase time
    walkGenerator_->t = 0;

    // 6.4 backup values
    forwardL0_ = forwardL_;
    forwardR0_ = forwardR_;
    turnRL0_ = turnRL_;
  }

  return supportChangedThisCycle;
}

KinematicMatrix UNSW2014Generator::calcFoot2TorsoFromOffsets(const float footSign,
                                                             const float footYawAngle,
                                                             const float legRollAngle,
                                                             const float footForwardOffset,
                                                             const float footHeight)
{
  assert(footSign == 1 || footSign == -1);

  const float compensatedTorsoOffset =
      torsoOffset_() + (enableTorsoCompensation_() ? getTorsoCompensationShift() : 0.f);

  return KinematicMatrix::transZ(-NaoProvider::link(LINKS::HIP_OFFSET_Z)) *           // hip2torso
         KinematicMatrix::transY(footSign * NaoProvider::link(LINKS::HIP_OFFSET_Y)) * // hipRoll2hip
         KinematicMatrix::rotX(-legRollAngle) * // upperLeg2hipRoll
         KinematicMatrix(
             Vector3f((-footForwardOffset - compensatedTorsoOffset) * mmPerM, // lowerLeg2upperLeg
                      0,                                                      //
                      -(walkHipHeight_() * mmPerM - NaoProvider::link(LINKS::FOOT_HEIGHT) -
                        footHeight * mmPerM) /
                          std::cos(legRollAngle))) *                      //
         KinematicMatrix::rotX(legRollAngle) *                            // footRoll2lowerLeg
         KinematicMatrix::rotZ(footSign * footYawAngle) *                 // yawedFootRoll2footRoll
         KinematicMatrix::transZ(-NaoProvider::link(LINKS::FOOT_HEIGHT)); // foot2yawedFootRoll
}

void UNSW2014Generator::calcFootOffsets(const float swingFootSign, const float forwardSwing0,
                                        const float forwardSupport0, float& forwardSwing,
                                        float& forwardSupport, float& leftSwing, float& leftSupport,
                                        float& footHeightSwing, float& footHeightSupport)
{
  if (weightShiftStatus_ == WeightShiftStatus::WEIGHT_DID_SHIFT)
  {
    // 5.3.1 forward: steps from (previous) -forward/2 to +forward/2, i.e. the target is
    // forward/2
    forwardSupport = forwardSupport0 + (forward_ / 2.f - forwardSupport0) *
                                           Range<float>::clipToZeroOne(
                                               walkGenerator_->t / walkGenerator_->stepDuration);
    forwardSwing = forwardSwing0 +
                   (-forward_ / 2.f - forwardSwing0) *
                       parabolicStep(walkGenerator_->t,
                                     walkGenerator_->stepDuration); // swing-foot follow-through

    // 5.3.4 left: steps from left0 to +left in one step and from (previous) -left to 0 in the
    // next
    float legLength = walkHipHeight_() - NaoProvider::link(LINKS::FOOT_HEIGHT) / mmPerM;
    float left0 = std::tan(-swingAngle_) * legLength;
    leftSupport = std::atan2(
        left0 + ((left_ * swingFootSign > 0 ? left_ : 0.f) - left0) *
                    Range<float>::clipToZeroOne(walkGenerator_->t / walkGenerator_->stepDuration),
        legLength);
    leftSwing =
        -std::atan2(left0 + ((left_ * swingFootSign > 0 ? left_ : 0.f) - left0) *
                                parabolicStep(walkGenerator_->t, walkGenerator_->stepDuration),
                    legLength);

    // 5.3.5 turn
    turnRL_ = turnRL0_ +
              ((turn_ * swingFootSign > 0 ? 1.f - insideTurnRatio_() : insideTurnRatio_()) *
                   swingFootSign * turn_ -
               turnRL0_) *
                  Range<float>::clipToZeroOne(walkGenerator_->t / walkGenerator_->stepDuration);
  }

  // 5.3.6 determine how high to lift the swing foot off the ground
  footHeightSwing =
      maxFootHeight_ *
      parabolicReturn(walkGenerator_->t / walkGenerator_->stepDuration); // lift swing foot
  footHeightSupport =
      maxFootHeight0_ *
      parabolicReturn(
          (switchPhase_ + walkGenerator_->t) /
          walkGenerator_->stepDuration); // return support foot to 0 if it was still lifted
}

Pose UNSW2014Generator::calcPredictedOdometryOffset(bool isLeftSwingFoot)
{
  // the change of the torso position relative to the support foot in x-direction over the last
  // cycle
  float forwardSupportOffset;
  // the change of the torso position relative to the support foot in y-direction over the last
  // cycle
  float leftSupportOffset;
  // the change of the torsos orientation  realtive to the support foot around the z-axis
  float turnSupportOffset;

  if (isLeftSwingFoot)
  {
    // In this case the right foot is the support foot. Under the assumption that the
    // the support foot does (barely) move relative to the ground, the change of the
    // torso pose can be modeled as the pose change of the torso relative to the foot
    forwardSupportOffset = forwardR_ - prevForwardR_;
    leftSupportOffset = (walkHipHeight_() - NaoProvider::link(LINKS::FOOT_HEIGHT) / mmPerM) *
                        (std::tan(leftR_) - std::tan(prevLeftR_));
    turnSupportOffset = 1.f * (turnRL_ - prevTurn_);
  }
  else
  {
    // the opposite of the above
    forwardSupportOffset = forwardL_ - prevForwardL_;
    leftSupportOffset = (walkHipHeight_() - NaoProvider::link(LINKS::FOOT_HEIGHT) / mmPerM) *
                        (std::tan(leftL_) - std::tan(prevLeftL_));
    turnSupportOffset = -1.f * (turnRL_ - prevTurn_);
  }
  // Work out incremental forward, left, and turn values for next time step
  Pose offset(forwardSupportOffset * odometryScale_().position.x(),
              leftSupportOffset * odometryScale_().position.y(),
              turnSupportOffset * odometryScale_().orientation);

  // backup values for next computation
  prevTurn_ = turnRL_;
  prevLeftL_ = leftL_;
  prevLeftR_ = leftR_;
  prevForwardL_ = forwardL_;
  prevForwardR_ = forwardR_;

  return offset;
}

Pose UNSW2014Generator::calcMeasuredOdometryOffset(bool isLeftSwingFoot,
                                                   bool supportChangedThisCycle)
{
  if (supportChangedThisCycle || walkState_ == WalkState::STARTING)
  {
    // if the support changed within this cycle we can not say how far we moved this time. Thus
    // we fall back to the predict.
    lastProjectedTorso2Support_ = getProjectedTorso2Support(isLeftSwingFoot);
    return {{lastProjectedTorsoShift_.x() * odometryScale_().position.x() * 0.001f,
             lastProjectedTorsoShift_.y() * odometryScale_().position.y() * 0.001f},
            0};
  }
  // Get the new projected torso
  Vector2f newProjectedTorsoPosition = getProjectedTorso2Support(isLeftSwingFoot);
  // Calculate the shift form the difference of the torso matrices
  Vector2f projectedShift = newProjectedTorsoPosition - lastProjectedTorso2Support_;
  lastProjectedTorso2Support_ = newProjectedTorsoPosition;
  lastProjectedTorsoShift_ = projectedShift;

  // Push back the odometry into the result queue
  return {{projectedShift.x() * odometryScale_().position.x() * 0.001f,
           projectedShift.y() * odometryScale_().position.y() * 0.001f},
          0};
}

Vector2f UNSW2014Generator::getProjectedTorso2Support(bool isLeftSwingFoot) const
{
  // Rotate with IMU measurement to take torso tilt into account
  const Vector3f& angle = imuSensorData_->angle;
  KinematicMatrix imu = KinematicMatrix::rotY(angle.y()) * KinematicMatrix::rotX(angle.x());
  // the position of the torso measured from the current support foot
  Vector3f measuredTorso2support =
      imu.invert() * (!isLeftSwingFoot ? robotKinematics_->matrices[JOINTS::L_FOOT].invert().posV
                                       : robotKinematics_->matrices[JOINTS::R_FOOT].invert().posV);
  return {measuredTorso2support.x(), measuredTorso2support.y()};
}

float UNSW2014Generator::calcWalkVolume(const float forward, const float left,
                                        const float turn) const
{
  return std::pow(std::pow(forward, walkVolumeTranslationExponent_()) +
                      std::pow(left, walkVolumeTranslationExponent_()),
                  (walkVolumeRotationExponent_() / walkVolumeTranslationExponent_())) +
         std::pow(turn, walkVolumeRotationExponent_());
}

bool UNSW2014Generator::ellipsoidClampWalk(const Pose& modifiedMaxSpeed,
                                           float modifiedMaxSpeedBackwards, float& forward,
                                           float& left, float& turn) const
{
  // Values in range [-1..1]
  float forwardAmount =
      forward / (forward >= 0.f ? modifiedMaxSpeed.position.x() : modifiedMaxSpeedBackwards);
  float leftAmount = left / modifiedMaxSpeed.position.y();
  float turnAmount = turn / modifiedMaxSpeed.orientation;

  float factor =
      std::max(std::max(std::abs(forwardAmount), std::abs(leftAmount)), std::abs(turnAmount));
  bool clamp = factor > 1.f;
  if (clamp)
  {
    forwardAmount /= factor;
    leftAmount /= factor;
    turnAmount /= factor;
  }
  // see if the point we are given is already inside the allowed walk params volume
  if (calcWalkVolume(std::abs(forwardAmount), std::abs(leftAmount), std::abs(turnAmount)) > 1.f)
  {
    clamp = true;
    float scale = 0.5f;
    float high = 1.f;
    float low = 0.f;

    // This is basically a binary search to find the point on the surface.
    for (unsigned i = 0; i < 10; ++i)
    {
      // give priority to turn. keep it the same
      if (calcWalkVolume(std::abs(forwardAmount) * scale, std::abs(leftAmount) * scale,
                         std::abs(turnAmount)) > 1.f)
      {
        high = scale;
      }
      else
      {
        low = scale;
      }
      scale = (low + high) / 2.f;
    }

    forwardAmount *= scale;
    leftAmount *= scale;
  }
  forward =
      (forward >= 0.f ? modifiedMaxSpeed.position.x() : modifiedMaxSpeedBackwards) * forwardAmount;
  left = modifiedMaxSpeed.position.y() * leftAmount;
  turn = modifiedMaxSpeed.orientation * turnAmount;
  return clamp;
}

float UNSW2014Generator::parabolicReturn(float f) const
{
  f = Range<float>::clipToZeroOne(f);

  if (f < 0.25f)
  {
    return 8.f * f * f;
  }
  else if (f < 0.75f)
  {
    float x = f - 0.5f;
    return 1.f - 8.f * x * x;
  }
  else
  {
    float x = 1.f - f;
    return 8.f * x * x;
  }
}

float UNSW2014Generator::parabolicStep(float time, float period) const
{
  float timeFraction = Range<float>::clipToZeroOne(time / period);
  if (timeFraction < 0.5f)
  {
    return 2.f * timeFraction * timeFraction;
  }
  else
  {
    return 4.f * timeFraction - 2.f * timeFraction * timeFraction - 1.f;
  }
}

float UNSW2014Generator::getTorsoCompensationShift() const
{
  /**
   * There is a distinction between step-wise compensation and cycle-wise compensation.
   *
   * Step-wise compensation depends on values that only change once per step (e.g. step size).
   * To ensure steadyness of the trajectory, step-wise compensations are lineary interpolated
   * between the steps.
   *
   * Cycle-wise compensation depends on values that change every cycle (e.g. some sensor
   * reading). These compensation offsets are NOT interpolated since steadyness is ensured by
   * inertia of the sensor (reduced bandwith by low pass filter).
   */
  return getStepwiseTorsoCompensation() + getCyclewiseTorsoCompensation();
}

float UNSW2014Generator::getCyclewiseTorsoCompensation() const
{
  float cycleWiseCompensation = 0.f;
  // head COMpensation - lean backwards when looking down since the head has a significant
  // weight
  const std::vector<KinematicMatrix> headMatrices = {
      {robotKinematics_->matrices[JOINTS::HEAD_YAW],
       robotKinematics_->matrices[JOINTS::HEAD_PITCH]}};
  const Vector3f headCom = Com::getComHead(headMatrices);
  cycleWiseCompensation += -headCom.x() / mmPerM * headComGain_();

  // arm COMpensation - lean backwards when having arms on the back
  if (armState_ != WalkGenerator::ArmState::NORMAL)
  {
    cycleWiseCompensation += -sin(nextArmAngles_[JOINTS_L_ARM::L_ELBOW_ROLL]) * armComGain_();
  }

  // accelerationCompensation - lean forward at positive acceleration
  // Rotate with IMU measurement to take torso tilt into account
  const Vector3f& bodyAngle2Ground = imuSensorData_->angle;
  KinematicMatrix body2Ground =
      KinematicMatrix::rotY(bodyAngle2Ground.y()) * KinematicMatrix::rotX(bodyAngle2Ground.x());

  const float filterAccX = (body2Ground * filteredAccelerometer_).x();
  cycleWiseCompensation += -filterAccX * accelerationCompensationGain_();
  return cycleWiseCompensation;
}

float UNSW2014Generator::getStepwiseTorsoCompensation() const
{
  float stepWiseCompensation = 0.f;
  // speedCompensation - lean forward at high speeds
  stepWiseCompensation += forward_ * speedCompensationGain_();
  // we want to the interpolation to be finished after half a step
  const float stepInterpolation =
      walkGenerator_->stepDuration == 0
          ? 1
          : Range<float>::clipToZeroOne(walkGenerator_->t / (0.5f * walkGenerator_->stepDuration));
  return stepWiseCompensation * stepInterpolation +
         lastStepwiseTorsoCompensation_ * (1.f - stepInterpolation);
}

void UNSW2014Generator::calculateBodyAnglesFromFootPoses(const KinematicMatrix& leftFoot,
                                                         const KinematicMatrix& rightFoot,
                                                         const bool isLeftSwing,
                                                         std::vector<float>& bodyAngles) const
{
  std::vector<float> lLegAngles, rLegAngles;
  // the support foot is the foot that is not swinging
  if (!isLeftSwing)
  {
    lLegAngles = InverseKinematics::getLLegAngles(leftFoot);
    rLegAngles = InverseKinematics::getFixedRLegAngles(rightFoot, lLegAngles[0]);
  }
  else
  {
    rLegAngles = InverseKinematics::getRLegAngles(rightFoot);
    lLegAngles = InverseKinematics::getFixedLLegAngles(leftFoot, rLegAngles[0]);
  }
  // assemble angles for the whole body
  for (int i = 0; i < JOINTS_L_LEG::L_LEG_MAX; i++)
  {
    bodyAngles[JOINTS::L_HIP_YAW_PITCH + i] = lLegAngles[i];
    bodyAngles[JOINTS::R_HIP_YAW_PITCH + i] = rLegAngles[i];
  }
}
