#pragma once

#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/KickConfigurationData.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/MotionPlannerOutput.hpp"
#include "Data/MotionRequest.hpp"
#include "Data/WalkGenerator.hpp"


/**
 * @brief WalkManState a wrapper to hold the shared state
 */
struct WalkManState
{
  WalkManState(const MotionActivation& ma, const MotionPlannerOutput& mpo, const MotionRequest& mr,
               const KickConfigurationData& kcd, const BodyPose& bp, const WalkGenerator& wg,
               const CycleInfo& ci, float minTimeInStand)
    : motionActivation(ma)
    , motionPlannerOutput(mpo)
    , motionRequest(mr)
    , kickConfigurationData(kcd)
    , bodyPose(bp)
    , walkGenerator(wg)
    , cycleInfo(ci)
    , minTimeInStandBeforeLeaving(minTimeInStand)
  {
  }

  /// passed content of external dependencies
  /// some information about which motion is currently active
  const MotionActivation& motionActivation;
  /// the request of the motion planner
  const MotionPlannerOutput& motionPlannerOutput;
  /// the unmodified request comming from brain
  const MotionRequest& motionRequest;
  /// some parameters to perform in walk kicks
  const KickConfigurationData& kickConfigurationData;
  /// some information about the body pose (fallen etc.)
  const BodyPose& bodyPose;
  /// the generator that can the walking joints
  const WalkGenerator& walkGenerator;
  /// some information about the timing of the current cycle
  const CycleInfo& cycleInfo;
  /// the minimum time we need to stand before we can start walking again
  const float minTimeInStandBeforeLeaving;

  /// some additional private members that are calculated depending on the state
  /// a function to calculate an offset to add to the poes of the swinging foot to create a kick
  /// motion
  std::function<KinematicMatrix(const float phase)> getKickFootOffset =
      std::function<KinematicMatrix(float)>();
  /// the speed that is requested from the walk generator
  Pose speed = Pose();
  /// the relative target in target mode
  Pose target = Pose();
  /// the relative direction we currently want to walk to
  Pose walkPathGradient = Pose();
  /// the last relative target
  Pose lastTarget = Pose();
  /// the last target processed
  TimePoint lastTimeWalking = TimePoint(0);
  /// the currently selected walk mode as understood by the generator
  WalkGenerator::WalkMode walkMode = WalkGenerator::WalkMode::VELOCITY_MODE;

  void setWalkParametersForVelocityMode(
      const Velocity& velocity,
      const std::function<KinematicMatrix(const float phase)>& getKickFootOffset =
          std::function<KinematicMatrix(float)>())
  {
    assert(!velocity.isPercentage());
    // in case of very small velocity requests in velocity mode we request clip to a small value
    // epsilon (for division by 0 reasons, and for the fact that we might want to perform an
    // inWalkKick with zero walk velocity). This will make the nao walk on the spot. If brain wants
    // to stand (instead of walk on a spot) it has to use the target mode.
    const float epsilon = 0.0000001f;
    speed = velocity.translation.norm() < epsilon ? Pose(epsilon, 0.f, velocity.rotation)
                                                  : Pose(velocity.translation, velocity.rotation);
    walkPathGradient = speed;
    walkMode = WalkGenerator::WalkMode::VELOCITY_MODE;
    this->getKickFootOffset = getKickFootOffset;
    lastTarget = Pose(10000.f, 10000.f);
  }

  void setWalkParametersForTargetMode(const Velocity& velocityComponentLimits, const Pose& target,
                                      const Pose& walkPathGradient)
  {
    assert(!velocityComponentLimits.isPercentage());
    speed = Pose(velocityComponentLimits.translation, velocityComponentLimits.rotation);
    this->walkPathGradient = walkPathGradient;
    if (target != lastTarget)
    {
      this->target = lastTarget = target;
    }
    walkMode = WalkGenerator::WalkMode::TARGET_MODE;
    this->getKickFootOffset = std::function<KinematicMatrix(float)>();
  }

  void setWalkParametersForStepSizeMode(
      const Pose& stepPoseOffset,
      const std::function<KinematicMatrix(const float phase)>& getKickFootOffset =
          std::function<KinematicMatrix(float)>())
  {
    const float epsilon = 0.0000001f;
    speed = stepPoseOffset.position.norm() < epsilon
                ? Pose(epsilon, 0.f, stepPoseOffset.orientation)
                : Pose(stepPoseOffset.position, stepPoseOffset.orientation);
    walkPathGradient = stepPoseOffset;
    walkMode = WalkGenerator::WalkMode::STEP_SIZE_MODE;
    this->getKickFootOffset = getKickFootOffset;
    lastTarget = Pose(10000.f, 10000.f);
  }

  void setWalkParametersForStand()
  {
    speed = Pose();
    walkMode = WalkGenerator::WalkMode::VELOCITY_MODE;
    getKickFootOffset = std::function<KinematicMatrix(float)>();
    lastTarget = Pose(10000.f, 10000.f);
  }
};
