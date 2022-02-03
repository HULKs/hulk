#include "Motion/Walking/WalkGenerator.hpp"
#include "Framework/Log/Log.hpp"
#include "Hardware/JointUtils.hpp"
#include "Tools/Math/Angle.hpp"
#include <cmath>
#include <limits>
#include <type_traits>

static constexpr float MM_PER_M = 1000.f;

WalkGenerator::WalkGenerator(const ModuleManagerInterface& manager)
  : Module{manager}
  , actionCommand_{*this}
  , bodyPose_{*this}
  , collisionDetectorData_{*this}
  , cycleInfo_{*this}
  , imuSensorData_{*this}
  , jointSensorData_{*this}
  , poses_{*this}
  , stepPlan_{*this}
  , walkManagerOutput_{*this}
  , walkGeneratorOutput_{*this}
  , baseWalkPeriod_{*this, "baseWalkPeriod", [] {}}
  , walkPeriodIncreaseFactor_{*this, "walkPeriodIncreaseFactor", [] {}}
  , walkHipHeight_{*this, "walkHipHeight", [] {}}
  , baseFootLift_{*this, "baseFootLift", [] {}}
  , footLiftIncreaseFactor_{*this, "footLiftIncreaseFactor", [] {}}
  , footLiftFirstStepFactor_{*this, "footLiftFirstStepFactor", [] {}}
  , supportSwitchPhaseRange_{*this, "supportSwitchPhaseRange", [] {}}
  , maxWeightShiftMisses_{*this, "maxWeightShiftMisses", [] {}}
  , emergencyStepSize_{*this, "emergencyStepSize", [] {}}
  , minSlowWeightShiftRatio_{*this, "minSlowWeightShiftRatio", [] {}}
  , maxSlowWeightShifts_{*this, "maxSlowWeightShifts", [] {}}
  , slowWeightShiftStandDelay_{*this, "slowWeightShiftStandDelay", [] {}}
  , torsoOffset_{*this, "torsoOffset", [] {}}
  , walkLegStiffness_{*this, "walkLegStiffness", [] {}}
  , standLegStiffness_{*this, "standLegStiffness", [] {}}
  , armStiffness_{*this, "armStiffness", [] {}}
  , armShoulderRoll_{*this, "armShoulderRoll", [this] { armShoulderRoll_() *= TO_RAD; }}
  , armShoulderRollIncreaseFactor_{*this, "armShoulderRollIncreaseFactor", [] {}}
  , armShoulderPitchFactor_{*this, "armShoulderPitchFactor", [] {}}
  , gyroLowPassRatio_{*this, "gyroLowPassRatio", [] {}}
  , gyroForwardBalanceFactor_{*this, "gyroForwardBalanceFactor", [] {}}
  , gyroBackwardBalanceFactor_{*this, "gyroBackwardBalanceFactor", [] {}}
  , gyroSidewaysBalanceFactor_{*this, "gyroSidewaysBalanceFactor", [] {}}
  , enableTorsoCompensation_{*this, "enableTorsoCompensation", [] {}}
  , armComGain_{*this, "armComGain", [] {}}
  , speedCompensationGain_{*this, "speedCompensationGain", [] {}}
  , enableGyroBalanceInStand_{*this, "enableGyroBalanceInStand", [] {}}
  , enableCollisionReaction_{*this, "enableCollisionReaction", [] {}}
  , triggerDebugCollision_{*this, "triggerDebugCollision", [] {}}
  , armLiftDuration_{*this, "armLiftDuration", [] {}}
  , armPullTightDuration_{*this, "armPullTightDuration", [] {}}
  , minTimeInStandBeforeLeaving_{*this, "minTimeInStandBeforeLeaving", [] {}}
  , maxDirectStandStepSize_{*this, "maxDirectStandStepSize",
                            [this] { maxDirectStandStepSize_().angle() *= TO_RAD; }}
  , armLiftAngles_{*this, "armLiftAngles", [] {}}
  , armPullTightAngles_{*this, "armPullTightAngles", [] {}}
{
  // initial unit conversions to rad
  maxDirectStandStepSize_().angle() *= TO_RAD;
  armShoulderRoll_() *= TO_RAD;

  walkGeneratorOutput_->baseWalkPeriod = baseWalkPeriod_();
}

void WalkGenerator::cycle()
{
  // filter the sensor data that is used for feedback
  filterSensorData();

  // produce velocity configurations
  walkGeneratorOutput_->maxVelocityComponents = stepPlan_->maxStepSize / baseWalkPeriod_();
  // always start with ready angles
  walkGeneratorOutput_->angles = poses_->angles[Poses::Type::READY];

  // do the ready pose when walking is not activated
  if (walkManagerOutput_->action == WalkManagerOutput::RequestAction::RESET)
  {
    resetGenerator();
  }
  if (!walkManagerOutput_->isActive)
  {
    walkGeneratorOutput_->stiffnesses.fill(0.7f);
    walkGeneratorOutput_->safeExit = true;
    walkGeneratorOutput_->valid = true;
    return;
  }
  if (walkGeneratorOutput_->t == 0.f)
  {
    // a new step begins
    debug().update(mount_ + ".stepBegin", *this);
    // update the state machine
    determineWalkState();
    // start a new step
    initializeStepStatesFromRequest(walkManagerOutput_->forward, walkManagerOutput_->left,
                                    walkManagerOutput_->turn);
  }
  if (walkState_ != WalkState::STANDING)
  {
    walkCycle();
  }
  calcJoints(walkManagerOutput_->getKickFootOffset);
  handleArms();
  balanceAdjustment();
  walkGeneratorOutput_->safeExit =
      cycleInfo_->getAbsoluteTimeDifference(lastTimeWalking_) > minTimeInStandBeforeLeaving_() &&
      walkGeneratorOutput_->armState == WalkGeneratorOutput::ArmState::NORMAL;
  walkGeneratorOutput_->requestedStepOffsets = Pose(forward_, left_, turn_);
  walkGeneratorOutput_->valid = true;
  debug().update(mount_ + ".eachCycle", *this);
}

void WalkGenerator::resetGenerator()
{
  walkGeneratorOutput_->stepDuration = 0.f;
  walkGeneratorOutput_->t = 0.f;
  walkState_ = WalkState::STANDING;
  forward_ = 0.f;
  forwardL_ = 0.f;
  forwardR_ = 0.f;
  forwardL0_ = 0.f;
  forwardR0_ = 0.f;
  left_ = 0.f;
  leftL_ = 0.f;
  leftR_ = 0.f;
  leftL0_ = 0.f;
  leftR0_ = 0.f;
  turnRL_ = 0.f;
  turnRL0_ = 0.f;
  footHeightL_ = 0.f;
  footHeightR_ = 0.f;
  switchPhase_ = 0.f;
  maxFootHeight_ = 0.f;
  maxFootHeight0_ = 0.f;
  weightShiftStatus_ = WeightShiftStatus::WEIGHT_DID_NOT_SHIFT;
  filteredGyroX_ = 0.f;
  filteredGyroY_ = 0.f;
  weightShiftMisses_ = 0.f;
  slowWeightShifts_ = 0.f;
}

void WalkGenerator::determineWalkState()
{
  // STOPPING and STARTING will only hold for one step
  if (walkState_ == WalkState::STARTING)
  {
    walkState_ = WalkState::WALKING;
  }
  else if (walkState_ == WalkState::STOPPING)
  {
    walkState_ = WalkState::STANDING;
  }
  else if (walkState_ != WalkState::WALKING &&
           walkManagerOutput_->action == WalkManagerOutput::RequestAction::WALK)
  {
    resetGenerator();
    walkState_ = WalkState::STARTING;
  }
  else if (walkState_ != WalkState::STANDING &&
           walkManagerOutput_->action == WalkManagerOutput::RequestAction::STAND)
  {
    if (std::abs(forwardL0_) <= maxDirectStandStepSize_().x() &&
        std::abs(forwardR0_) <= maxDirectStandStepSize_().x() &&
        std::abs(leftL0_) <= maxDirectStandStepSize_().y() &&
        std::abs(leftR0_) <= maxDirectStandStepSize_().y() &&
        std::abs(turnRL0_) <= maxDirectStandStepSize_().angle())
    {
      walkState_ = WalkState::STANDING;
    }
    else
    {
      walkState_ = WalkState::STOPPING;
    }
  }
}

void WalkGenerator::calcFootOffsets()
{
  // Calculate intra-walkphase forward, left and turn at time-step dt
  if (walkGeneratorOutput_->isLeftPhase)
  {
    calcFootOffsets(forwardL0_, forwardR0_, &forwardL_, &forwardR_, leftL0_, leftR0_, &leftL_,
                    &leftR_, &footHeightL_, &footHeightR_);
  }
  else
  {
    calcFootOffsets(forwardR0_, forwardL0_, &forwardR_, &forwardL_, leftR0_, leftL0_, &leftR_,
                    &leftL_, &footHeightR_, &footHeightL_);
  }

  // Special conditions when priming the walk
  if (walkState_ == WalkState::STARTING)
  {
    // reduce max lift due to short duration
    footHeightL_ *= footLiftFirstStepFactor_();
    footHeightR_ *= footLiftFirstStepFactor_();
  }
}
void WalkGenerator::calcJoints(const std::function<KinematicMatrix(float phase)>& getKickFootOffset)
{
  // Now assemble the kinematic matrices for both feet from the calculations above.
  // This also adds compensation and calibration.
  KinematicMatrix leftFoot2Torso =
      calcFoot2TorsoFromOffsets(1, turnRL_, leftL_, forwardL_, footHeightL_);
  KinematicMatrix rightFoot2Torso =
      calcFoot2TorsoFromOffsets(-1, turnRL_, leftR_, forwardR_, footHeightR_);
  // 9.2 Walk kicks
  if (getKickFootOffset)
  {
    // TODO: I think the order of multiplication is correct here. However, it's quite late right
    // now. If things are weird, recheck the order
    (walkGeneratorOutput_->isLeftPhase ? leftFoot2Torso : rightFoot2Torso) *= (getKickFootOffset(
        std::min(walkGeneratorOutput_->t / walkGeneratorOutput_->stepDuration, 1.f)));
  }
  // Inverse kinematics
  walkGeneratorOutput_->angles = calculateBodyAnglesFromFootPoses(
      leftFoot2Torso, rightFoot2Torso, walkGeneratorOutput_->isLeftPhase);
  // Set joint values and stiffness
  const float legStiffness =
      walkState_ == WalkState::STANDING ? standLegStiffness_() : walkLegStiffness_();
  // set the default stiffness for all joints
  walkGeneratorOutput_->stiffnesses.fill(legStiffness);
}

void WalkGenerator::handleArms()
{
  // use a lower stiffness for the arms
  JointsArmArray<float> armStiffnesses;
  armStiffnesses.fill(armStiffness_());
  JointUtils::fillArms(walkGeneratorOutput_->stiffnesses, armStiffnesses, armStiffnesses);
  // 10.1 Arms
  const bool collisionPredicted =
      triggerDebugCollision_() || collisionDetectorData_->collisionLeftRigid ||
      collisionDetectorData_->collisionRightRigid || collisionDetectorData_->duelRigid;

  // if the arms are still in normal mode and collisionPredicted is approaching we move them back
  if (enableCollisionReaction_() && collisionPredicted && walkGeneratorOutput_->t == 0 &&
      armState_ == WalkGeneratorOutput::ArmState::NORMAL && bodyPose_->footContact &&
      (actionCommand_->body().type == ActionCommand::Body::MotionType::WALK ||
       actionCommand_->body().type == ActionCommand::Body::MotionType::STAND))
  {
    // reset the arm interpolators to move the arms back
    armStageOneInterpolator_.reset(jointSensorData_->getBodyAngles(), armLiftAngles_(),
                                   armLiftDuration_());
    armStageTwoInterpolator_.reset(armLiftAngles_(), armPullTightAngles_(),
                                   armPullTightDuration_());
    armState_ = WalkGeneratorOutput::ArmState::MOVING_BACK;
  }
  else if ((!enableCollisionReaction_() || !collisionPredicted || !bodyPose_->footContact ||
            (actionCommand_->body().type != ActionCommand::Body::MotionType::WALK &&
             actionCommand_->body().type != ActionCommand::Body::MotionType::STAND)) &&
           walkGeneratorOutput_->t == 0 && armState_ == WalkGeneratorOutput::ArmState::BACK)
  {
    // rest the arm interplator to move the arms to the front
    armStageOneInterpolator_.reset(jointSensorData_->getBodyAngles(), armLiftAngles_(),
                                   armPullTightDuration_());
    armStageTwoInterpolator_.reset(armLiftAngles_(), poses_->angles[Poses::Type::READY],
                                   armLiftDuration_());
    armState_ = WalkGeneratorOutput::ArmState::MOVING_FRONT;
  }

  JointsArray<float> nextArmAngles{poses_->angles[Poses::Type::READY]};
  switch (armState_)
  {
    case WalkGeneratorOutput::ArmState::MOVING_FRONT:
      [[fallthrough]];
    case WalkGeneratorOutput::ArmState::MOVING_BACK:
      // if we are currently in a transition from front to back or vice versa, we simply continue
      // with that interpolation
      if (!armStageOneInterpolator_.isFinished())
      {
        static_assert(std::is_same_v<Clock::duration::period, std::chrono::seconds::period>);
        nextArmAngles = {armStageOneInterpolator_.step(cycleInfo_->cycleTime.count())};
      }
      else if (!armStageTwoInterpolator_.isFinished())
      {
        static_assert(std::is_same_v<Clock::duration::period, std::chrono::seconds::period>);
        nextArmAngles = {armStageTwoInterpolator_.step(cycleInfo_->cycleTime.count())};
      }
      else
      {
        assert(false);
      }
      break;
    case WalkGeneratorOutput::ArmState::NORMAL:
      // "natural" arm swing while walking to counterbalance foot swing
      nextArmAngles[Joints::L_SHOULDER_PITCH] = 90 * TO_RAD + forwardL_ * armShoulderPitchFactor_();
      nextArmAngles[Joints::R_SHOULDER_PITCH] = 90 * TO_RAD + forwardR_ * armShoulderPitchFactor_();
      nextArmAngles[Joints::L_SHOULDER_ROLL] =
          armShoulderRoll_() +
          std::max(std::abs(leftL_), std::abs(leftR_)) * armShoulderRollIncreaseFactor_();
      nextArmAngles[Joints::R_SHOULDER_ROLL] = -nextArmAngles[Joints::L_SHOULDER_ROLL];
      break;
    case WalkGeneratorOutput::ArmState::BACK:
      nextArmAngles = armPullTightAngles_();
      break;
    default:
      assert(false && "Unhandled WalkGeneratorOutput::ArmState");
      break;
  }

  if (armStageTwoInterpolator_.isFinished() &&
      (armState_ == WalkGeneratorOutput::ArmState::MOVING_FRONT ||
       armState_ == WalkGeneratorOutput::ArmState::MOVING_BACK))
  {
    // the current arm motion is finished thus we can advance the state
    armState_ = static_cast<WalkGeneratorOutput::ArmState>((static_cast<int>(armState_) + 1) % 4);
  }

  JointUtils::fillArms(walkGeneratorOutput_->angles, JointUtils::extractLeftArm(nextArmAngles),
                       JointUtils::extractRightArm(nextArmAngles));
  walkGeneratorOutput_->armState = armState_;
}


void WalkGenerator::balanceAdjustment()
{
  // Sagittal balance
  // adjust ankle pitch in proportion to filtered gyroY
  const float adjustment = filteredGyroY_ * (filteredGyroY_ > 0 ? gyroForwardBalanceFactor_()
                                                                : gyroBackwardBalanceFactor_());
  if (walkState_ == WalkState::STANDING && enableGyroBalanceInStand_())
  {
    walkGeneratorOutput_->angles[Joints::R_ANKLE_PITCH] += adjustment;
    walkGeneratorOutput_->angles[Joints::L_ANKLE_PITCH] += adjustment;
  }
  else if (walkState_ == WalkState::WALKING)
  {
    walkGeneratorOutput_->angles[walkGeneratorOutput_->isLeftPhase ? Joints::R_ANKLE_PITCH
                                                                   : Joints::L_ANKLE_PITCH] +=
        adjustment;
  }
  // Lateral balance
  if (walkState_ == WalkState::STANDING && enableGyroBalanceInStand_())
  {
    const float balanceAdjustment = filteredGyroX_ * gyroSidewaysBalanceFactor_();
    walkGeneratorOutput_->angles[Joints::L_ANKLE_ROLL] += balanceAdjustment;
    walkGeneratorOutput_->angles[Joints::R_ANKLE_ROLL] += balanceAdjustment;
  }
}

void WalkGenerator::filterSensorData()
{
  filteredGyroY_ = gyroLowPassRatio_() * filteredGyroY_ +
                   (1.f - gyroLowPassRatio_()) * imuSensorData_->gyroscope.y();
  filteredGyroX_ = gyroLowPassRatio_() * filteredGyroX_ +
                   (1.f - gyroLowPassRatio_()) * imuSensorData_->gyroscope.x();
}

void WalkGenerator::initializeStepStatesFromRequest(const float forward, const float left,
                                                    const float turn)
{
  Pose request{forward, left, turn};

  walkGeneratorOutput_->isLeftPhase = bodyPose_->supportSide < 0;
  if (weightShiftStatus_ == WeightShiftStatus::EMERGENCY_STEP)
  {
    request = Pose{
        0.f, walkGeneratorOutput_->isLeftPhase ? emergencyStepSize_() : -emergencyStepSize_(), 0.f};
    weightShiftStatus_ = WeightShiftStatus::WEIGHT_DID_SHIFT;
    // force a phase change
    walkGeneratorOutput_->isLeftPhase = !walkGeneratorOutput_->isLeftPhase;
  }
  else if (walkState_ == WalkState::STARTING)
  {
    // don't move when starting
    request = Pose{};
    // make first real step in direction of movement
    walkGeneratorOutput_->isLeftPhase = left_ < 0.f;
  }
  else if (cycleInfo_->getAbsoluteTimeDifference(timeWhenSlowWeightShiftsDetected_) <=
           slowWeightShiftStandDelay_())
  {
    // do a stopping (hopefully balancing) step, if slow weight shifts are detected
    request = Pose{};
  }

  forward_ = request.x();
  left_ = request.y();
  turn_ = request.angle();

  if (walkState_ == WalkState::STANDING)
  {
    walkGeneratorOutput_->stepDuration = 0.f;
  }
  else
  {
    walkGeneratorOutput_->stepDuration = (baseWalkPeriod_()                                       //
                                          + walkPeriodIncreaseFactor_().x() * std::abs(forward_)  //
                                          + walkPeriodIncreaseFactor_().y() * std::abs(left_)     //
                                          + walkPeriodIncreaseFactor_().angle() * std::abs(turn_) //
    );
  }

  // 5.1 Calculate the height to lift each swing foot
  maxFootHeight_ = baseFootLift_()                                      //
                   + std::abs(forward_) * footLiftIncreaseFactor_().x() //
                   + std::abs(left_) * footLiftIncreaseFactor_().y()    //
                   + std::abs(turn_) * footLiftIncreaseFactor_().angle();
}

void WalkGenerator::walkCycle()
{
  // set the last time walking to now
  lastTimeWalking_ = cycleInfo_->startTime;
  // Advance the timer if we are walking
  static_assert(std::is_same_v<Clock::duration::period, std::chrono::seconds::period>);
  walkGeneratorOutput_->t += cycleInfo_->cycleTime.count();
  calcFootOffsets();
  // calculate the return offset
  if (walkGeneratorOutput_->isLeftPhase)
  {
    walkGeneratorOutput_->returnOffset = Pose{forwardL_, leftL_, turnRL_};
  }
  else
  {
    walkGeneratorOutput_->returnOffset = Pose{forwardR_, leftR_, -turnRL_};
  }
  // Changing support foot. Note isLeftPhase means left foot is swing foot.
  // t>0.75*T tries to avoid bounce, especially when side-stepping
  // lastZMPL*ZMPL<0.0 indicates that support foot has changed
  // t>3*T tries to get out of "stuck" situations
  const bool supportChangedInTime =
      walkGeneratorOutput_->t >
          supportSwitchPhaseRange_().min * walkGeneratorOutput_->stepDuration &&
      bodyPose_->supportChanged;
  const bool stepPhaseTookTooLong =
      walkGeneratorOutput_->t > supportSwitchPhaseRange_().max * walkGeneratorOutput_->stepDuration;
  // a step phase ends if the support foot changed (after MINIMUM half of the expected step
  // duration) or if the step took too long and we want to force the end
  if (supportChangedInTime || stepPhaseTookTooLong)
  {
    handleSupportPhaseEnd();
    // reset step phase time, a new step can begin next cycle
    walkGeneratorOutput_->t = 0;
  }
}

void WalkGenerator::handleSupportPhaseEnd()
{
  lastStepwiseTorsoCompensation_ = getStepwiseTorsoCompensation();
  switchPhase_ = walkGeneratorOutput_->t;
  // backup values
  maxFootHeight0_ = maxFootHeight_;
  forwardL0_ = forwardL_;
  forwardR0_ = forwardR_;
  leftL0_ = leftL_;
  leftR0_ = leftR_;
  turnRL0_ = turnRL_;
  weightShiftStatus_ = walkGeneratorOutput_->isLeftPhase != (bodyPose_->supportSide < 0)
                           ? WeightShiftStatus::WEIGHT_DID_SHIFT
                           : WeightShiftStatus::WEIGHT_DID_NOT_SHIFT;

  debug().update(mount_ + ".stepEnd", *this);
  if (weightShiftStatus_ == WeightShiftStatus::WEIGHT_DID_NOT_SHIFT)
  {
    if (++weightShiftMisses_ > maxWeightShiftMisses_())
    {
      Log<M_MOTION>(LogLevel::INFO) << "WalkGenerator: Too many weight shift misses";
      weightShiftStatus_ = WeightShiftStatus::EMERGENCY_STEP;
      weightShiftMisses_ = 0;
    }
  }
  else
  {
    // weight shifted again. Reset the counter
    weightShiftMisses_ = 0;
    if (switchPhase_ > minSlowWeightShiftRatio_() * walkGeneratorOutput_->stepDuration)
    {
      if (++slowWeightShifts_ > maxSlowWeightShifts_())
      {
        Log<M_MOTION>(LogLevel::INFO) << "WalkGenerator: Too many slow weight shifts";
        timeWhenSlowWeightShiftsDetected_ = cycleInfo_->startTime;
      }
    }
    else
    {
      slowWeightShifts_ = 0;
    }
  }
}

KinematicMatrix WalkGenerator::calcFoot2TorsoFromOffsets(const float footSign, const float turnRL,
                                                         const float left, const float forward,
                                                         const float footHeight)
{
  assert(footSign == 1 || footSign == -1);

  const float compensatedTorsoOffset =
      torsoOffset_() + (enableTorsoCompensation_() ? getTorsoCompensationShift() : 0.f);

  const float legLength = walkHipHeight_() - robotMetrics().link(Links::FOOT_HEIGHT) / MM_PER_M;
  const float legRollAngle = std::atan2(left, legLength);

  return
      // yawedFootRoll2footRoll
      KinematicMatrix::rotZ(footSign * turnRL)
      // hip2torso
      * KinematicMatrix::transZ(-robotMetrics().link(Links::HIP_OFFSET_Z))
      // hipRoll2hip
      * KinematicMatrix::transY(footSign * robotMetrics().link(Links::HIP_OFFSET_Y))
      // upperLeg2hipRoll
      * KinematicMatrix::rotX(legRollAngle)
      // lowerLeg2upperLeg
      *
      KinematicMatrix(Vector3f((forward - compensatedTorsoOffset) * MM_PER_M, 0,
                               -(walkHipHeight_() * MM_PER_M -
                                 robotMetrics().link(Links::FOOT_HEIGHT) - footHeight * MM_PER_M) /
                                   std::cos(legRollAngle)))
      // footRoll2lowerLeg
      * KinematicMatrix::rotX(-legRollAngle)
      // foot2yawedFootRoll
      * KinematicMatrix::transZ(-robotMetrics().link(Links::FOOT_HEIGHT));
}

void WalkGenerator::calcFootOffsets(const float forwardSwing0, const float forwardSupport0,
                                    float* forwardSwing, float* forwardSupport,
                                    const float leftSwing0, const float leftSupport0,
                                    float* leftSwing, float* leftSupport, float* footHeightSwing,
                                    float* footHeightSupport)
{
  const float relativeTimeInStep =
      std::clamp(walkGeneratorOutput_->t / walkGeneratorOutput_->stepDuration, 0.f, 1.f);
  if (weightShiftStatus_ == WeightShiftStatus::WEIGHT_DID_SHIFT)
  {
    const float parabolicTimeInStep = parabolicStep(relativeTimeInStep);
    *forwardSupport = forwardSupport0 + (-forward_ / 2.f - forwardSupport0) * relativeTimeInStep;
    *forwardSwing = forwardSwing0 + (forward_ / 2.f - forwardSwing0) * parabolicTimeInStep;

    *leftSupport = leftSupport0 + (-left_ / 2.f - leftSupport0) * relativeTimeInStep;
    *leftSwing = leftSwing0 + (left_ / 2.f - leftSwing0) * parabolicTimeInStep;

    // 5.3.5 turn
    const float turnRL{walkGeneratorOutput_->isLeftPhase ? turn_ : -1.f * turn_};
    turnRL_ = turnRL0_ + (turnRL / 2.f - turnRL0_) * relativeTimeInStep;
  }

  // 5.3.6 determine how high to lift the swing foot off the ground
  // lift swing foot
  *footHeightSwing = maxFootHeight_ * parabolicReturn(relativeTimeInStep);
  // return support foot to 0 if it was still lifted
  *footHeightSupport =
      maxFootHeight0_ *
      parabolicReturn(std::clamp(
          switchPhase_ / walkGeneratorOutput_->stepDuration + relativeTimeInStep, 0.f, 1.f));
}

float WalkGenerator::parabolicReturn(const float f)
{
  assert(f >= 0.f && f <= 1.f);
  if (f < 0.25f)
  {
    return 8.f * f * f;
  }
  if (f < 0.75f)
  {
    float x = f - 0.5f;
    return 1.f - 8.f * x * x;
  }
  float x = 1.f - f;
  return 8.f * x * x;
}

float WalkGenerator::parabolicStep(const float f)
{
  assert(f >= 0.f && f <= 1.f);
  if (f < 0.5f)
  {
    return 2.f * f * f;
  }
  return 4.f * f - 2.f * f * f - 1.f;
}

float WalkGenerator::getTorsoCompensationShift() const
{
  /**
   * There is a distinction between step-wise compensation and cycle-wise compensation.
   *
   * Step-wise compensation depends on values that only change once per step (i.e. step size).
   * To ensure steadyness of the trajectory, step-wise compensations are lineary interpolated
   * between the steps.
   *
   * Cycle-wise compensation depends on values that change every cycle (e.g. some sensor
   * reading). These compensation offsets are NOT interpolated since steadyness is ensured by
   * inertia of the sensor (reduced bandwith by low pass filter).
   */
  return getStepwiseTorsoCompensation() + getCyclewiseTorsoCompensation();
}

float WalkGenerator::getCyclewiseTorsoCompensation() const
{
  // arm COMpensation - lean backwards when having arms on the back
  return armState_ != WalkGeneratorOutput::ArmState::NORMAL
             ? -std::sin(jointSensorData_->getLArmAngles()[JointsArm::ELBOW_ROLL]) * armComGain_()
             : 0.f;
}

float WalkGenerator::getStepwiseTorsoCompensation() const
{
  // speedCompensation - lean forward at high speeds
  const float stepWiseCompensation = forward_ * speedCompensationGain_();
  // we want to the interpolation to be finished after half a step
  const float stepInterpolation =
      walkGeneratorOutput_->stepDuration == 0
          ? 1
          : std::clamp(walkGeneratorOutput_->t / (0.5f * walkGeneratorOutput_->stepDuration), 0.f,
                       1.f);
  return stepWiseCompensation * stepInterpolation +
         lastStepwiseTorsoCompensation_ * (1.f - stepInterpolation);
}

JointsArray<float> WalkGenerator::calculateBodyAnglesFromFootPoses(const KinematicMatrix& leftFoot,
                                                                   const KinematicMatrix& rightFoot,
                                                                   const bool isLeftSwing) const
{
  JointsArray<float> angles{};
  // the support foot is the foot that is not swinging
  if (!isLeftSwing)
  {
    const auto lLegAngles = inverseKinematics().getLLegAngles(leftFoot);
    const auto rLegAngles =
        inverseKinematics().getFixedRLegAngles(rightFoot, lLegAngles[JointsLeg::HIP_YAW_PITCH]);
    JointUtils::fillLegs(angles, lLegAngles, rLegAngles);
  }
  else
  {
    const auto rLegAngles = inverseKinematics().getRLegAngles(rightFoot);
    const auto lLegAngles =
        inverseKinematics().getFixedLLegAngles(leftFoot, rLegAngles[JointsLeg::HIP_YAW_PITCH]);
    JointUtils::fillLegs(angles, lLegAngles, rLegAngles);
  }
  return angles;
}
