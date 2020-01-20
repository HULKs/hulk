#include "Tools/Math/Eigen.hpp"
#include <Modules/Debug/Debug.h>
#include <Tools/Chronometer.hpp>
#include <Tools/Math/Angle.hpp>
#include <Tools/Math/Line.hpp>
#include <Tools/Math/Random.hpp>

#include "UKFPositionKnowledge.hpp"
#include "print.h"

UKFPositionKnowledge::UKFPositionKnowledge(const ModuleManagerInterface& manager)
  : Module(manager)
  , sigmaInitial_(*this, "sigmaInitial", [] {})
  , sigmaPenalized_(*this, "sigmaPenalized", [] {})
  , filterProcessNoise_(*this, "filterProcessNoise", [] {})
  , predictProcessNoiseFraction_(*this, "predictProcessNoiseFraction", [] {})
  , hypothesisSelectionHysteresis_(*this, "hypothesisSelectionHysteresis", [] {})
  , absoluteEvalThreshold_(*this, "absoluteEvalThreshold", [] {})
  , relativeEvalThreshold_(*this, "relativeEvalThreshold", [] {})
  , mergeRadius_(*this, "mergeRadius", [] {})
  , mergeAngle_(*this, "mergeAngle", [] {})
  , maxPSOPenaltySpotAssociationDistance_(*this, "maxPSOPenaltySpotAssociationDistance", [] {})
  , startAnywhereAtSidelines_(*this, "startAnywhereAtSidelines", [] {})
  , maxNumberOfHypotheses_(*this, "maxNumberOfHypotheses", [] {})
  , useSensorResetting_(*this, "useSensorResetting", [] {})
  , useInternalCircleDetection_(*this, "useInternalCircleDetection", [] {})
  , ignoreCirclePerceptsNearGoalSupport_(*this, "ignoreCirclePerceptsNearGoalSupport", [] {})
  , ignorePenaltyAreasWithoutOrientation_(*this, "ignorePenaltyAreasWithoutOrientation", [] {})
  , alwaysUseMultiplePenaltyShootoutPositions_(*this, "alwaysUseMultiplePenaltyShootoutPositions",
                                               [] {})
  , strikerLocalizeInPSO_(*this, "strikerLocalizeInPSO", [] {})
  , cycleInfo_(*this)
  , fieldDimensions_(*this)
  , odometryOffset_(*this)
  , cameraMatrix_(*this)
  , gameControllerState_(*this)
  , playerConfiguration_(*this)
  , landmarkModel_(*this)
  , bodyPose_(*this)
  , motionState_(*this)
  , jointSensorData_(*this)
  , imageData_(*this)
  , robotPosition_(*this)
  , lastPose_()
  , fieldInfo_(*playerConfiguration_, *fieldDimensions_)
  , positionProvider_(*this, fieldInfo_, *gameControllerState_, *playerConfiguration_,
                      *landmarkModel_, *fieldDimensions_)
  , poseHypotheses_()
  , lastState_(GameState::INITIAL)
  , lastPenalty_(Penalty::NONE)
  , useMeasurements_(false)
  , resettingIsAllowed_(false)
  , resettedThisCycle_(false)
  , jumpedThisCycle_(true)
  , wasHighInSet_(false)
{
  preparePoseHypotheses(maxNumberOfHypotheses_(), sigmaInitial_(),
                        [this](unsigned int& clusterHint) -> Pose {
                          return positionProvider_.getInitial(clusterHint, false);
                        });
}

void UKFPositionKnowledge::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  resettedThisCycle_ = false;
  jumpedThisCycle_ = false;

  // Handle game state changes (integrating knowledge from rules etc.)
  updateState();
  // Predict the hypothesis movement based on odometry updates
  odometryPredict();
  // update the hypotheses with percepted information from the environment
  measurementUpdate();
  // merge hypotheses that are close enough to eachother
  mergeHypotheses();
  // generate new hypotheses (e.g. from sensor resetting)
  generateNewHypotheses();
  // evaluate the validity of the hypothesis based on covariance and some knowledge about valid
  // poses
  evaluateHypotheses();
  // publish the best hypothesis
  publishPoseEstimate();

  // Stream data to debugging tool
  debug().update(mount_, *this);
}

void UKFPositionKnowledge::updateState()
{
  // In case of penalty shootout
  if (gameControllerState_->gamePhase == GamePhase::PENALTYSHOOT)
  {
    // Penalty Shootout requires special handling. The robot is in SET when waiting.
    // It will be switched to PLAYING when the robot should start to move.
    if ((gameControllerState_->gameState == GameState::PLAYING &&
         lastState_ != GameState::PLAYING) ||
        (gameControllerState_->penalty == Penalty::NONE && lastPenalty_ != Penalty::NONE) ||
        gameControllerState_->gameState == GameState::SET)
    {
      // if we are penalty taker and in general PSO-Competition-Mode (5 different PSO positions
      // around the penalty spot, we need 5 hypotheses)
      if (gameControllerState_->kickingTeam && alwaysUseMultiplePenaltyShootoutPositions_())
      {
        // if we are the kicking team and in general PSO mode, there are 5 positions where we can be
        // (spread around the penalty spot)
        preparePoseHypotheses(6, sigmaInitial_(), [this](unsigned int& clusterHint) -> Pose {
          return positionProvider_.getPenaltyShootout(clusterHint, true, false);
        });
      }
      else
      {
        // No special PSO mode or keeper. There is only one postion that we can be placed at.
        preparePoseHypotheses(1, sigmaInitial_(), [this](unsigned int& clusterHint) -> Pose {
          return positionProvider_.getPenaltyShootout(clusterHint, false, false);
        });
      }
    }
  }
  else
  {
    // The robot just got unpenalized
    if (gameControllerState_->penalty == Penalty::NONE && lastPenalty_ != Penalty::NONE)
    {
      // As the robot is unpenalized, all hypotheses need to be droped, because any preveous
      // knowledge is not valid any more
      // Always call manual placement entire team after motion in set!!
      if (gameControllerState_->gameState == GameState::SET ||
          lastPenalty_ == Penalty::ILLEGAL_MOTION_IN_SET)
      {
        // Robots that are unpenalized during SET are manually placed.
        preparePoseHypotheses(5, sigmaPenalized_(), [this](unsigned int& clusterHint) -> Pose {
          return positionProvider_.getManuallyPlaced(clusterHint, false);
        });
      }
      else
      {
        // Robot is unpenalized during a normal game situation and starts from either of the
        // penalized positions
        preparePoseHypotheses(2, sigmaPenalized_(), [this](unsigned int& clusterHint) -> Pose {
          return positionProvider_.getPenalized(clusterHint, false);
        });
      }
    }
    else if ((gameControllerState_->gameState == GameState::INITIAL &&
              lastState_ != GameState::INITIAL) ||
             (gameControllerState_->gameState == GameState::READY &&
              lastState_ == GameState::INITIAL))
    {
      // reset for the next set phase
      wasHighInSet_ = false;
      // start fom initial pose (somewhere at the side line in the own half)
      int intialNumberOfHypotheses = startAnywhereAtSidelines_() ? maxNumberOfHypotheses_() : 1;
      preparePoseHypotheses(intialNumberOfHypotheses, sigmaInitial_(),
                            [this](unsigned int& clusterHint) -> Pose {
                              return positionProvider_.getInitial(clusterHint, false);
                            });
    }
    else if (gameControllerState_->gameState == GameState::SET)
    {
      if ((!bodyPose_->footContact || wasHighInSet_) &&
          (motionState_->bodyMotion == MotionRequest::BodyMotion::STAND ||
           motionState_->bodyMotion == MotionRequest::BodyMotion::PENALIZED))
      {
        wasHighInSet_ = true;
        // Robot got picked up (lost ground contact) during set, thus it has been manually placed
        // They will constantly reset their hypotheses since they might be picket up by the referee
        // again to adjust the placement (this acually happens quite often)
        preparePoseHypotheses(5, sigmaPenalized_(), [this](unsigned int& clusterHint) -> Pose {
          return positionProvider_.getManuallyPlaced(clusterHint, false);
        });
      }
    }
    // whenever we change from SET to playing and we were high in SET we claim to be manually
    // placed. We cann now reset wasHighInSet_ for the next SET phase.
    if (gameControllerState_->gameState == GameState::PLAYING && lastState_ == GameState::SET)
    {
      // If the gamestate changed (after it has changed!) from SET to PLAYING
      if (wasHighInSet_)
      {
        // robots that were high in set were picked up by the referee. Hypotheses are placed at all
        // valid maually placement positions
        preparePoseHypotheses(5, sigmaPenalized_(), [this](unsigned int& clusterHint) -> Pose {
          return positionProvider_.getManuallyPlaced(clusterHint, false);
        });
        // reset wasHighInSet_ (e.g. for second half)
        wasHighInSet_ = false;
      }
      // Note: We don't reset hypotheses that are in the opponents half at the beginning of playing.
      // If we made it to the opponents half and were not picked up (no FSR-Reading) then the
      // referee apparently did not notice. Thus stay calm and start playing.
    }
  }

  /*
   * We want to use measurements, only if we are not penalized and sufficiently stable.
   * In penalty shootout, measurements are only taken if in addition to the aforementioned criteria
   * localization is enabled explicitly or is needed due to challenge mode (multi-penalty-shootout
   * positions)
   */
  const bool sufficientlyStable = (motionState_->bodyMotion == MotionRequest::BodyMotion::WALK ||
                                   motionState_->bodyMotion == MotionRequest::BodyMotion::STAND) &&
                                  !bodyPose_->wonky;

  const bool inMultiPSOMode = alwaysUseMultiplePenaltyShootoutPositions_();
  const bool localizeInPenaltyShootout =
      gameControllerState_->kickingTeam && (strikerLocalizeInPSO_() || inMultiPSOMode);

  useMeasurements_ =
      gameControllerState_->penalty == Penalty::NONE && sufficientlyStable &&
      (gameControllerState_->gamePhase != GamePhase::PENALTYSHOOT || localizeInPenaltyShootout);

  lastState_ = gameControllerState_->gameState;
  lastPenalty_ = gameControllerState_->penalty;
}

void UKFPositionKnowledge::preparePoseHypotheses(
    const unsigned int numberOfHypotheses, const Vector3f& sigma,
    const std::function<Pose(unsigned int& clusterID)>& poseProviderFunc)
{
  poseHypotheses_.clear();
  positionProvider_.resetHypothesesCounter();
  poseHypotheses_.resize(numberOfHypotheses, PoseHypothesis(*this, *fieldDimensions_, fieldInfo_));
  unsigned int id = 0;
  for (auto& poseHypothesis : poseHypotheses_)
  {
    unsigned int clusterHint = 0;
    Pose pose = poseProviderFunc(clusterHint);
    // The hypotheses are initialized with weight -1 since they need to be evaluated at least once
    poseHypothesis.reset(pose, sigma, -1.f, id);
    id++;
  }

  bestPoseHypothesisIt_ = poseHypotheses_.begin();
  resettedThisCycle_ = true;
}

void UKFPositionKnowledge::odometryPredict()
{
  for (auto& poseHypothesis : poseHypotheses_)
  {
    poseHypothesis.odometryPredict(
        odometryOffset_->odometryOffset,
        {filterProcessNoise_().x() * filterProcessNoise_().x(),
         filterProcessNoise_().y() * filterProcessNoise_().y(),
         filterProcessNoise_().z() * filterProcessNoise_().z()},
        {predictProcessNoiseFraction_().x() * predictProcessNoiseFraction_().x(),
         predictProcessNoiseFraction_().y() * predictProcessNoiseFraction_().y(),
         predictProcessNoiseFraction_().z() * predictProcessNoiseFraction_().z()});
  }
}

bool UKFPositionKnowledge::linesAreValid() const
{
  const float headYawAbs = std::fabs(jointSensorData_->angles[JOINTS::HEAD_YAW]);
  return (imageData_->camera == Camera::TOP || // One can use all images of the top camera
          (imageData_->camera == Camera::BOTTOM &&
           headYawAbs < 40.f * TO_RAD)); // avoid shoulder with the bottom camera
}

void UKFPositionKnowledge::measurementUpdate()
{
  // Early return if using measurements is not allowed (e.g. when fallen)
  if (!useMeasurements_)
  {
    return;
  }

  // use all the measurements to update the state estimates of each hypothesis
  for (auto& poseHypothesis : poseHypotheses_)
  {
    // Update the hypotheses with the lines
    poseHypothesis.updateWithSetOfLines(landmarkModel_->filteredLines, cameraMatrix_->camera2ground,
                                        useInternalCircleDetection_());

    // Update hypotheses with center circle
    for (auto& centerCircle : landmarkModel_->centerCircles)
    {
      poseHypothesis.updateWithCenterCircle(centerCircle, cameraMatrix_->camera2ground,
                                            ignoreCirclePerceptsNearGoalSupport_());
    }

    // Update hypotheses with penalty spots
    for (auto& penaltyArea : landmarkModel_->penaltyAreas)
    {
      // ignore penalty areas / penalty spots in penalty shoot out if the
      // projected position from the current state mean is too far off the
      // expected position (penaltySpot[1] is the penalty spot of the opponent)
      if (gameControllerState_->gamePhase == GamePhase::PENALTYSHOOT &&
          (poseHypothesis.getPoseMean() * penaltyArea.position - fieldInfo_.penaltySpots[1])
                  .norm() > maxPSOPenaltySpotAssociationDistance_())
      {
        continue;
      }
      poseHypothesis.updateWithPenaltyArea(penaltyArea, cameraMatrix_->camera2ground,
                                           ignorePenaltyAreasWithoutOrientation_());
    }
  }
}

TimePoint UKFPositionKnowledge::updateLastTimeJumped(const Pose& currentPoseEstimate)
{
  // Calculate the last time jumped
  const float jumpDistThreshSquared = 0.5f * 0.5f;
  const float jumpAngleThresh = 30 * TO_RAD;
  if ((currentPoseEstimate.position - lastPose_.position).squaredNorm() > jumpDistThreshSquared ||
      Angle::angleDiff(currentPoseEstimate.orientation, lastPose_.orientation) > jumpAngleThresh)
  {
    lastTimeJumped_ = cycleInfo_->startTime;
    jumpedThisCycle_ = true;
  }
  lastPose_ = currentPoseEstimate;

  return lastTimeJumped_;
}

void UKFPositionKnowledge::publishPoseEstimate()
{
  // For now there is only one poseHypothesis anyway
  robotPosition_->pose = bestPoseHypothesisIt_->getPoseMean();
  // update last time jumped with the selected pose
  robotPosition_->lastTimeJumped = updateLastTimeJumped(robotPosition_->pose);

  const bool moreThanOneHypothesis = poseHypotheses_.size() > 1;
  const bool isPenalized = gameControllerState_->penalty != Penalty::NONE;
  const bool penaltyShootout = gameControllerState_->gamePhase == GamePhase::PENALTYSHOOT;
  // Check if there was an irregularity that makes the pose unreliable
  const bool recentIrregularity = resettedThisCycle_ || jumpedThisCycle_ ||
                          (wasHighInSet_ && gameControllerState_->gameState == GameState::SET);
  // pose is valid if number of hypotheses is one, the pose was not reset, did not jump, the
  // robot was not high in set and we don't have a penalty
  // pose is valid in penalty shoot out if theres only one pose hypothesis
  const bool invalid = moreThanOneHypothesis || ((recentIrregularity || isPenalized) && !penaltyShootout);
  robotPosition_->valid = !invalid;
}

void UKFPositionKnowledge::mergeHypotheses()
{
  // Merging is only possible, if there's more then one hypothesis
  if (poseHypotheses_.size() == 1)
  {
    return;
  }
  Vector2f eps = {mergeRadius_(), mergeAngle_()};

  for (auto itCurrent = poseHypotheses_.begin(); itCurrent != poseHypotheses_.end(); ++itCurrent)
  {
    if (itCurrent->getMeanEvalError() < 0.f)
    {
      continue;
    }
    // backup the posehyptheses in advance to always compare to original list
    const PoseHypothesis unchangedCurrent = *itCurrent;

    // Avoid duplicate comparison by starting at std::next(itCurrent)
    for (auto itCompare = std::next(itCurrent); itCompare != poseHypotheses_.end();)
    {
      // Check if hypotheses are mergable
      if (unchangedCurrent.isInNeighbourhood(*itCompare, eps))
      {
        // If mergable, update the current pose with the compare-pose
        itCurrent->poseSensorUpdate(itCompare->getStateMean(), itCompare->getStateCov());
        // handling the case of merging the beset hypothesis
        if (bestPoseHypothesisIt_ == itCompare)
        {
          bestPoseHypothesisIt_ = itCurrent;
        }
        // remove the merged one from the list
        itCompare = poseHypotheses_.erase(itCompare);
      }
      else
      {
        ++itCompare;
      }
    }
  }
}

void UKFPositionKnowledge::generateNewHypotheses()
{
  if (!useMeasurements_ || !resettingIsAllowed_ || poseHypotheses_.size() > 1 ||
      !useSensorResetting_())
  {
    // don't reset if resetting is not allowed or association could be ambiguous
    return;
  }
  // TODO: implement sensor resetting
}

void UKFPositionKnowledge::evaluateHypotheses()
{
  if (!useMeasurements_)
  {
    return;
  }

  for (auto& poseHypothesis : poseHypotheses_)
  {
    poseHypothesis.evaluate(landmarkModel_->filteredLines);
  }

  // Handle case of hypotheses leaving the field and search for the best hypothesis at the same time
  // (without hysteresis)
  bool droppedBestHypothesis = false;
  // hypothesisWithLowestError is initialized with the last element in order to ensure that this
  // will always contain a pose (even after dropping some), since we never drop the last hypothesis
  auto hypothesisWithLowestError = std::prev(poseHypotheses_.end());
  // we only need to do this if there is more then one hypothesis since we never drop the last one
  for (auto itPoseHypothesis = poseHypotheses_.begin(); itPoseHypothesis != poseHypotheses_.end();)
  {
    if (poseHypotheses_.size() <= 1)
    {
      break;
    }
    if (!fieldDimensions_->isInsideCarpet(itPoseHypothesis->getPoseMean().position))
    {
      // check if we erased the best hypothesis
      if (itPoseHypothesis == bestPoseHypothesisIt_)
      {
        droppedBestHypothesis = true;
      }
      itPoseHypothesis = poseHypotheses_.erase(itPoseHypothesis);
    }
    else
    {
      const float itError = itPoseHypothesis->getMeanEvalError();
      const float bestError = hypothesisWithLowestError->getMeanEvalError();
      if (itError > 0.f && (bestError < 0.f || itError > bestError))
      {
        hypothesisWithLowestError = itPoseHypothesis;
      }
      ++itPoseHypothesis;
    }
  }

  // we never delete the last pose. Thus this should never happen.
  assert(poseHypotheses_.size() > 0);

  // Finding the pose with the lowest error (with hysteresis) and selecting it as the best
  // hypothesis
  if (droppedBestHypothesis)
  {
    // if we dropped the best hypothesis in the step before we take the sorting result without
    // hysteresis
    bestPoseHypothesisIt_ = hypothesisWithLowestError;
  }
  else
  {
    // if the best hypothesis was not dropped in the last step, we search for the best with
    // hysteresis
    for (auto itPoseHypothesis = poseHypotheses_.begin(); itPoseHypothesis != poseHypotheses_.end();
         ++itPoseHypothesis)
    {
      const float itError = itPoseHypothesis->getMeanEvalError();
      const float bestError = bestPoseHypothesisIt_->getMeanEvalError();
      // find the best hypothesis that has been evaluated before and is significantly better then
      // the best from the last cycle
      if (itError > 0.f &&
          (bestError < 0.f || itError < bestError * (1 - hypothesisSelectionHysteresis_())))
      {
        bestPoseHypothesisIt_ = itPoseHypothesis;
      }
    }
  }
  // TODO: Handle the case of even the best hypothesis having a high error (e.g. request sensor
  // resetting)
  // Remove hypotheses that are significantly worse then the best
  assert(relativeEvalThreshold_() >= 1.f);
  for (auto itPoseHypothesis = poseHypotheses_.begin(); itPoseHypothesis != poseHypotheses_.end();)
  {
    const float itError = itPoseHypothesis->getMeanEvalError();
    const float bestError = bestPoseHypothesisIt_->getMeanEvalError();
    if (bestError > 0.f && itError > 0.f && itError > relativeEvalThreshold_() * bestError &&
        itError > absoluteEvalThreshold_())
    {
      itPoseHypothesis = poseHypotheses_.erase(itPoseHypothesis);
    }
    else
    {
      ++itPoseHypothesis;
    }
  }
}

void UKFPositionKnowledge::toValue(Uni::Value& value) const
{
  value = Uni::Value(Uni::ValueType::OBJECT);
  value["poseHypotheses"] << poseHypotheses_;
  value["publishedPose"] << bestPoseHypothesisIt_->getPoseMean();
}
