#include <cmath>

#include "Modules/NaoProvider.h"
#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Geometry.hpp"
#include "Tools/Math/Line.hpp"
#include "Tools/Math/Random.hpp"

#include "ParticlePositionKnowledge.hpp"


ParticlePositionKnowledge::ParticlePositionKnowledge(const ModuleManagerInterface& manager)
  : Module(manager)
  , sigma_(*this, "sigma", [] {})
  , sigmaPrediction_(*this, "sigmaPrediction", [] {})
  , numberOfParticles_(*this, "numberOfParticles", [] {})
  , maxConsideredLineMeasurements_(*this, "maxConsideredLineMeasurements", [] {})
  , lineAssociationDistance_(*this, "lineAssociationDistance", [] {})
  , lineAssociationAngle_(*this, "lineAssociationAngle",
                          [this] { lineAssociationAngle_() *= TO_RAD; })
  , maxGyroNormWhenMeasuring_(*this, "maxGyroNormWhenMeasuring", [] {})
  , sensorResettingFraction_(*this, "sensorResettingFraction", [] {})
  , transmitParticles_(*this, "transmitParticles", [] {})
  , alwaysUseMultiplePenaltyShootoutPositions_(*this, "alwaysUseMultiplePenaltyShootoutPositions",
                                               [] {})
  , strikerLocalizeInPSO_(*this, "strikerLocalizeInPSO", [] {})
  , cycleInfo_(*this)
  , goalData_(*this)
  , gameControllerState_(*this)
  , playerConfiguration_(*this)
  , landmarkModel_(*this)
  , bodyPose_(*this)
  , motionState_(*this)
  , odometryOffset_(*this)
  , fieldDimensions_(*this)
  , imuSensorData_(*this)
  , jointSensorData_(*this)
  , robotPosition_(*this)
  , fieldInfo_(*playerConfiguration_, *fieldDimensions_)
  , positionProvider_(*this, fieldInfo_, *gameControllerState_, *playerConfiguration_,
                      *landmarkModel_, *fieldDimensions_)
  , particles_()
  , pose_()
  , lastPose_()
  , lastTimeJumped_()
  , lastState_(GameState::INITIAL)
  , lastPenalty_(Penalty::NONE)
  , useMeasurements_(false)
  , updatedWithMeasurements_(false)
  , resettedWeightsThisCycle_(false)
  , wasHighInSet_(false)
{
  for (unsigned int i = 0; i < static_cast<unsigned int>(numberOfParticles_()); i++)
  {
    // Only one cluster since all particles are currently distributed around the same pose.
    unsigned int clusterIndex;
    Pose pose(positionProvider_.getInitial(clusterIndex));
    particles_.emplace_back(pose, clusterIndex);
  }
  lineAssociationAngle_() *= TO_RAD;
  nextClusterID_ = 2;
  resetWeights();
}

void ParticlePositionKnowledge::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  resettedWeightsThisCycle_ = false;
  jumpedThisCycle_ = false;
  updatedWithMeasurements_ = false;
  updateState();
  integrateOdometry();
  if (useMeasurements_)
  {
    integrateMeasurements();
  }
  evaluateParticle();
  computePosition();
  if (updatedWithMeasurements_)
  {
    resample();
    resetWeights();
  }
  // update last time jumped if pose significantly changed
  updateLastTimeJumped();
  robotPosition_->pose = pose_;
  robotPosition_->lastTimeJumped = lastTimeJumped_;
  // Handle reset to initial of the GameController state.
  if (gameControllerState_->gameState == GameState::INITIAL && wasHighInSet_)
  {
    wasHighInSet_ = false;
  }
  if (resettedWeightsThisCycle_ || jumpedThisCycle_ ||
      (wasHighInSet_ && gameControllerState_->gameState == GameState::SET))
  {
    robotPosition_->valid = false;
  }
  else
  {
    robotPosition_->valid = true;
  }
  debug().update(mount_, *this);
}

void ParticlePositionKnowledge::updateState()
{
  if (gameControllerState_->gamePhase == GamePhase::PENALTYSHOOT)
  {
    // Penalty Shootout requires special handling. The robot is in SET when waiting.
    // It will be switched to PLAYING when the robot should start to move.
    if ((gameControllerState_->gameState == GameState::PLAYING && lastState_ != GameState::PLAYING) ||
        (gameControllerState_->penalty == Penalty::NONE && lastPenalty_ != Penalty::NONE))
    {
      // we are in multi PSO mode if required by the gamecontroller or configured
      const bool inMultiPSOMode =
          alwaysUseMultiplePenaltyShootoutPositions_() ||
          gameControllerState_->type == CompetitionType::GENERAL_PENALTY_KICK;
      // All particles are replaced with particles that correspond to the positions according to the
      // rules.
      for (auto& particle : particles_)
      {
        unsigned int clusterID;
        // There are six possible states in a penalty shootout due to new rules 2018. Each particle
        // gets its own cluster.
        Pose pose(positionProvider_.getPenaltyShootout(clusterID, inMultiPSOMode));
        particle.replace(pose, clusterID);
      }
      nextClusterID_ = 5;
      resetWeights();
      resettedWeightsThisCycle_ = true;
    }
  }
  else
  {
    if (gameControllerState_->penalty == Penalty::NONE && lastPenalty_ != Penalty::NONE &&
        lastPenalty_ != Penalty::ILLEGAL_MOTION_IN_SET)
    {
      if (gameControllerState_->gameState == GameState::SET)
      {
        // Robots that are unpenalized during SET are manually placed.
        for (auto& particle : particles_)
        {
          unsigned int clusterID;
          // There are two penalized positions, and since we don't know exactly which is returned,
          // each particle gets its own cluster.
          Pose pose(positionProvider_.getManuallyPlaced(clusterID));
          particle.replace(pose, clusterID);
        }
        nextClusterID_ = 5;
      }
      else
      {
        // All particles are being replaced by particles near the penalize position as written in
        // the config.
        for (auto& particle : particles_)
        {
          unsigned int clusterID;
          // There are two penalized positions, and since we don't know exactly which is returned,
          // each particle gets its own cluster.
          Pose pose(positionProvider_.getPenalized(clusterID));
          particle.replace(pose, clusterID);
        }
        nextClusterID_ = 2;
      }
      resetWeights();
      resettedWeightsThisCycle_ = true;
    }
    else if ((gameControllerState_->gameState == GameState::INITIAL &&
              lastState_ != GameState::INITIAL) ||
             (gameControllerState_->gameState == GameState::READY && lastState_ == GameState::INITIAL))
    {
      // All particles are being replaced by particles near the initial position of
      // the given player number (as in config). Robot position is not valid(ated) in this state.
      for (auto& particle : particles_)
      {
        unsigned int clusterID;
        // There is currently only one initial position, so each particle gets the same cluster ID.
        Pose pose(positionProvider_.getInitial(clusterID));
        particle.replace(pose, clusterID);
      }
      nextClusterID_ = 2; // There are at most 2 initial clusters.
      resetWeights();
      resettedWeightsThisCycle_ = true;
    }
    else if (gameControllerState_->gameState == GameState::PLAYING && lastState_ == GameState::SET)
    {
      // If the gamestate changed (after it has changed!) from SET to PLAYING
      if (wasHighInSet_)
      {
        for (auto& particle : particles_)
        {
          unsigned int clusterID;
          Pose pose(positionProvider_.getManuallyPlaced(clusterID));
          particle.replace(pose, clusterID);
        }
        nextClusterID_ = 5;
        resetWeights();
        resettedWeightsThisCycle_ = true;
        wasHighInSet_ = false;
      }
      else
      {
        for (auto& particle : particles_)
        {
          unsigned int clusterID;
          if (particle.pose.position.x() > 0.f ||
              (!gameControllerState_->kickingTeam &&
               particle.pose.position.norm() <
                   (fieldDimensions_->fieldCenterCircleDiameter * 0.5f - 0.2f)))
          {
            // Particle should be replaced. It is not possible to start a game
            // in the opponent's half or outside the field.
            // There are generally multiple manual placement positions, so each particle gets its
            // own cluster ID.
            Pose pose(positionProvider_.getManuallyPlaced(clusterID));
            particle.replace(pose, nextClusterID_ + clusterID);
            particle.weight = 1.f / numberOfParticles_();
          }
        }
        nextClusterID_ += 5;
      }
    }
    else if (gameControllerState_->gameState == GameState::SET)
    {
      if ((!bodyPose_->footContact || wasHighInSet_) &&
          (motionState_->bodyMotion == MotionRequest::BodyMotion::STAND))
      {
        wasHighInSet_ = true;
        for (auto& particle : particles_)
        {
          unsigned int clusterID;
          Pose pose(positionProvider_.getManuallyPlaced(clusterID));
          particle.replace(pose, clusterID);
        }
        nextClusterID_ = 5;
        resetWeights();
        resettedWeightsThisCycle_ = true;
      }
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
                                  imuSensorData_->gyroscope.norm() < maxGyroNormWhenMeasuring_();

  const bool inMultiPSOMode = gameControllerState_->type == CompetitionType::GENERAL_PENALTY_KICK ||
                              alwaysUseMultiplePenaltyShootoutPositions_();

  const bool localizeInPenaltyShootout =
      gameControllerState_->kickingTeam && (strikerLocalizeInPSO_() || inMultiPSOMode);

  useMeasurements_ =
      gameControllerState_->penalty == Penalty::NONE && sufficientlyStable &&
      (gameControllerState_->gamePhase != GamePhase::PENALTYSHOOT || localizeInPenaltyShootout);

  lastState_ = gameControllerState_->gameState;
  lastPenalty_ = gameControllerState_->penalty;
}

void ParticlePositionKnowledge::integrateOdometry()
{
  const Pose& odometry = odometryOffset_->odometryOffset;
  const auto distanceMoved =
      Vector2f(std::abs(odometry.position.x()), std::abs(odometry.position.y()));
  const auto distanceRotated = std::abs(odometry.orientation);
  const float constantFactor = 0.00001;

  Vector3f sigma(constantFactor + distanceMoved.x() * sigmaPrediction_().x(),
                 constantFactor + distanceMoved.y() * sigmaPrediction_().y(),
                 constantFactor + distanceRotated * sigmaPrediction_().z());
  for (auto& particle : particles_)
  {
    // Add noise to model the inaccuracies of the odometry.
    Pose noisyOffset = positionProvider_.addGaussianNoise(odometry, sigma);
    predictParticleMovement(particle, noisyOffset);
  }
}

void ParticlePositionKnowledge::integrateMeasurements()
{
  if (!landmarkModel_->filteredLines.empty() && lastLineTimestamp_ != landmarkModel_->timestamp)
  {
    lastLineTimestamp_ = landmarkModel_->timestamp;

    for (size_t i = 0; i < landmarkModel_->filteredLines.size(); ++i)
    {
      if (static_cast<int>(i) >= maxConsideredLineMeasurements_())
      {
        break;
      }
      auto& lineInfo = landmarkModel_->filteredLineInfos[i];
      auto& projectionDistance = lineInfo.projectionDistance;
      // update each line
      for (auto& particle : particles_)
      {
        updateWithLine(particle, *lineInfo.line, projectionDistance);
      }

      updatedWithMeasurements_ = true;
    }
  }
}

void ParticlePositionKnowledge::resample()
{
  std::vector<PositionParticle> savedParticles = particles_;
  // Normalize particle weights.
  float weightSum = 0;
  for (auto& it : savedParticles)
  {
    weightSum += it.weight;
  }
  debug().update("ParticlePositionKnowledge.weightSum", weightSum);
  // Do not resample if the weight sum is too low.
  // if one uses only 10 lines, each measurment causes a weight of minimum 0.1 => 10e-10
  // with 100 particles this results in 10e-8
  if (weightSum < 10e-5)
  {

    // Reset all particles to random poses.
    nextClusterID_ = 0;
    for (auto& particle : particles_)
    {
      // Random poses are not related to each other, so each one gets an own cluster ID.
      particle.replace(positionProvider_.addGaussianNoise(
                           pose_, {sigma_().x() * 10.f, sigma_().y() * 10.f, sigma_().z()}),
                       nextClusterID_++);
    }

    resetWeights();
    resettedWeightsThisCycle_ = true;
    return;
  }
  for (auto& it : savedParticles)
  {
    it.weight /= weightSum;
  }
  // The keeper is not allowed to use resetting at all
  bool useSensorResetting =
      (positionProvider_.isSensorResettingAvailable() && playerConfiguration_->playerNumber != 1);
  unsigned int keepParticles = numberOfParticles_();
  if (useSensorResetting)
  {
    // If particles are replaced, some less particles should be kept.
    keepParticles *= (1.f - sensorResettingFraction_());
  }
  // Do stochastic universal sampling (SUS).
  float p = 1.f / keepParticles;
  float currentSum = Random::uniformFloat(0, p);
  weightSum = 0;
  particles_.clear();
  for (auto& oldParticle : savedParticles)
  {
    weightSum += oldParticle.weight;
    while (currentSum < weightSum && particles_.size() < keepParticles)
    {
      particles_.emplace_back(positionProvider_.addGaussianNoise(oldParticle.pose, sigma_()),
                              oldParticle.clusterID);
      currentSum += p;
    }
  }
  if (useSensorResetting)
  {
    // calculate score to chose resetting postion:
    float maxPositionDiff =
        fieldDimensions_->fieldLength * 0.5f + fieldDimensions_->fieldBorderStripWidth;

    Pose resettingPose = positionProvider_.getSensorResetting();
    Pose mirroredResettingPose = {-resettingPose.position,
                                  static_cast<float>(resettingPose.orientation + M_PI)};

    float resettingPositionDiff =
        (resettingPose.position - mirroredResettingPose.position).squaredNorm();
    const float minResettingPositionDiff =
        fieldDimensions_->fieldLength * fieldDimensions_->fieldLength * 0.0625;

    float resettingScore = 0.f;
    float mirroredResettingScore = 0.f;

    const float angleDiff = Angle::angleDiff(pose_.orientation, resettingPose.orientation);
    const float angleScore = 1.f - angleDiff / M_PI;

    const float positionDiff = (pose_.position - resettingPose.position).norm();
    const float positionScore = 1.f - positionDiff / maxPositionDiff;

    const float mirroredAngleDiff =
        Angle::angleDiff(pose_.orientation, mirroredResettingPose.orientation);
    const float mirroredAngleScore = 1.f - mirroredAngleDiff / M_PI;

    const float mirroredPositionDiff = (pose_.position - resettingPose.position).norm();
    const float mirroredPositionScore = 1.f - mirroredPositionDiff / maxPositionDiff;

    // Check whether the resetting positions are sufficiently differ in position (not close to
    // center)
    if (resettingPositionDiff < minResettingPositionDiff)
    {
      // if the restting pose is to close to the center, only reset by orientation (relying on the
      // imu sensor fusion)
      resettingScore = angleScore;
      mirroredResettingScore = mirroredAngleScore;
    }
    else
    {
      // calculate a total score form angle and position score
      resettingScore = 0.8f * angleScore + 0.2f * positionScore;
      mirroredResettingScore = 0.8f * mirroredAngleScore + 0.2f * mirroredPositionScore;
    }

    // select the resetting pose by score
    if (resettingScore > mirroredResettingScore)
    {
      for (unsigned int i = 0; i < numberOfParticles_() - keepParticles; i++)
      {
        particles_.emplace_back(positionProvider_.addGaussianNoise(resettingPose, sigma_()),
                                nextClusterID_++);
      }
    }
    else
    {
      for (unsigned int i = 0; i < numberOfParticles_() - keepParticles; i++)
      {
        particles_.emplace_back(positionProvider_.addGaussianNoise(mirroredResettingPose, sigma_()),
                                nextClusterID_++);
      }
    }
  }
  // TODO: Maybe merge clusters.
}

void ParticlePositionKnowledge::computePosition()
{
  // Find the cluster with the greatest sum of particle weights.
  // unordered_map guarantees that new elements of type float are initialized with 0.
  // Maybe this is not very efficient.
  std::unordered_map<unsigned int, float> clusterWeights;
  unsigned int bestClusterID = 0;
  float bestWeightSum = 0;
  for (auto& particle : particles_)
  {
    float& currentWeight = clusterWeights[particle.clusterID];
    currentWeight += particle.weight;
    if (currentWeight > bestWeightSum)
    {
      bestClusterID = particle.clusterID;
      bestWeightSum = currentWeight;
    }
  }
  if (bestWeightSum == 0)
  {
    // If something went wrong keep the pose from the last cycle.
    return;
  }
  // Compute the CoM of the best cluster.
  Vector2f position = Vector2f::Zero();
  Vector2f direction = Vector2f::Zero();
  for (auto& particle : particles_)
  {
    if (particle.clusterID == bestClusterID)
    {
      position += particle.pose.position * particle.weight;
      // This is needed to compute some kind of mean of angles.
      // Since the mean of an angle of 359deg and 1deg should be 0deg and not 180deg, weighted
      // direction vectors are summed.
      direction +=
          Vector2f(std::cos(particle.pose.orientation), std::sin(particle.pose.orientation)) *
          particle.weight;
    }
  }
  position /= bestWeightSum;
  direction /= bestWeightSum;
  pose_ = Pose(position, atan2(direction.y(), direction.x()));
}

void ParticlePositionKnowledge::resetWeights()
{
  // Set all particle weights to an equal value.
  float w = 1.f / numberOfParticles_();
  for (auto& it : particles_)
  {
    it.weight = w;
  }
}

void ParticlePositionKnowledge::evaluateParticle()
{
  // Positions outside the carpet should be impossible. One could think of replacing the sample
  // (sensor resetting) instead of setting its weight to 0.
  for (auto& particle : particles_)
  {
    if (!fieldDimensions_->isInsideCarpet(particle.pose.position))
    {
      particle.weight = 0;
    }
    /*
     * @TODO We cannot depend on the role here, as the roles are depending on the position.
     * It's crappy, but why are we doing this anyway?
     * Why aren't we choosing the weight based on the distance to the prior position?
     */
    if (playerConfiguration_->playerNumber == 1 &&
        gameControllerState_->gamePhase != GamePhase::PENALTYSHOOT)
    {
      if (particle.pose.position.x() > 0)
      {
        particle.weight = 0;
      }
    }
  }
}

void ParticlePositionKnowledge::updateLastTimeJumped()
{
  // Calculate the last time jumped
  const float jumpDistThreshSquared = 0.5f * 0.5f;
  const float jumpAngleThresh = 30 * TO_RAD;
  if ((pose_.position - lastPose_.position).squaredNorm() > jumpDistThreshSquared ||
      Angle::angleDiff(pose_.orientation, lastPose_.orientation) > jumpAngleThresh)
  {
    lastTimeJumped_ = cycleInfo_->startTime;
    jumpedThisCycle_ = true;
  }
  lastPose_ = pose_;
}

void ParticlePositionKnowledge::toValue(Uni::Value& value) const
{
  value = Uni::Value(Uni::ValueType::OBJECT);
  if (transmitParticles_())
  {
    std::vector<PositionParticle> particlesToTransmit;

    for (unsigned int k = 0; k < particles_.size(); k += particles_.size() / 20)
    {
      particlesToTransmit.push_back(particles_[k]);
    }

    value["particles"] << particlesToTransmit;
  }
  value["pose"] << pose_;
}

void ParticlePositionKnowledge::predictParticleMovement(PositionParticle& particle,
                                                        const Pose& pose)
{
  particle.pose += pose;
}

void ParticlePositionKnowledge::updateWithLine(PositionParticle& particle, const Line<float>& line,
                                               const float projectionDistance)
{
  // TODO: Actually this should be the outer loop (iterate over the particles in the inner loop)
  Line<float> worldLine(particle.pose * line.p1, particle.pose * line.p2);
  particle.weight *= weightByLine(worldLine, projectionDistance);
}

void ParticlePositionKnowledge::updateWithLandMarkPosition(
    PositionParticle& particle, const Vector2f& relativeLandmarkMeasurement,
    const float measurementVariance, const Vector2f& absoluteGroundTruthPosition)
{
  const Vector2f relativeGroundTruthPosition =
      particle.pose.inverse() * absoluteGroundTruthPosition;
  const float squaredError =
      (relativeLandmarkMeasurement - relativeGroundTruthPosition).squaredNorm();
  const float weightByFieldMarkMeasurement = std::exp(-0.5f * squaredError / measurementVariance);

  particle.weight *= weightByFieldMarkMeasurement;
}

void ParticlePositionKnowledge::updateWithGoalPosts(PositionParticle& particle,
                                                    const VecVector2f& goalPosts)
{
  if (goalPosts.empty())
  {
    return;
  }

  for (auto& goalPost : goalPosts)
  {
    Vector2f worldPost(particle.pose * goalPost);

    particle.weight *= weightByGoalPost(worldPost);
  }
}

float ParticlePositionKnowledge::angleBetweenLineVectors(const Vector2f& v1,
                                                         const Vector2f& v2) const
{
  const float a1 = std::atan2(v1.y(), v1.x());
  const float a2 = std::atan2(v2.y(), v2.x());

  float phi = std::fmod(std::abs(a1 - a2), M_PI);
  phi = phi > M_PI_2 ? M_PI - phi : phi;
  return phi;
}

float ParticlePositionKnowledge::weightByLine(const Line<float>& line,
                                              const float projectionDistance) const
{
  const Vector2f lineVector = line.p1 - line.p2;
  const Vector2f lineCenterPoint = (line.p1 + line.p2) * 0.5f;
  const float lineLength = lineVector.squaredNorm();
  // TODO: Maybe use this later for early return (then one should preclassify the field lines)
  // float lineOrientation = atan2(lineVector.y, lineVector.x);

  // storing the error and length of the best line
  float bestFieldLineError = 1.f;

  for (auto& fieldLine : fieldInfo_.lines)
  {
    const Vector2f fieldLineVector = fieldLine.p1 - fieldLine.p2;
    const float fieldLineLength = fieldLineVector.squaredNorm();
    // Check the orthogonal distance of the endpoints to the line
    float error = 0;
    float distCenter = Geometry::distPointToLineSegment(fieldLine, lineCenterPoint);
    // Check if line was associated with center circle
    bool associatedWithCircle =
        (fieldLine.p1.x() != fieldLine.p2.x()) && (fieldLine.p1.y() != fieldLine.p2.y());
    float lineAngleDiff = angleBetweenLineVectors(lineVector, fieldLineVector);
    // Drop lines that are far off
    if (distCenter > lineAssociationDistance_() ||
        ((lineAngleDiff > lineAssociationAngle_() || lineLength > 1.2f * fieldLineLength) &&
         !associatedWithCircle))
    {
      continue;
    }
    // For center circle lines, only the distance is checked
    if (associatedWithCircle)
    {
      error = distCenter / lineAssociationDistance_();
    }
    else
    {
      error = distCenter / lineAssociationDistance_() * 0.5f + lineAngleDiff / M_PI_2 * 0.5f;
    }
    if (error < bestFieldLineError)
    {
      bestFieldLineError = error;
    }
  }
  // The line could not be associated
  if (bestFieldLineError == 1.f)
  {
    // TODO: Reason about this
    return 0.1;
  }

  float weight = 1.1 - bestFieldLineError;

  // scale with projection distance
  const float projectionWeight = 1 - projectionDistance / landmarkModel_->maxLineProjectionDistance;
  // if the projection distance is the projectionWeight drops to 0. Thus the error of a far away
  // line has a low impact on the total weight
  return std::pow(weight, projectionWeight);
}

float ParticlePositionKnowledge::weightByGoalPost(const Vector2f& goalPost) const
{
  /*
   * This code has the following effect:
   * If no matching post is found, the weight is 0.33.
   * If the post matches exactly, the weight is 1.
   * Between that, the weight is assigned in a hyperbolic function.
   * A post is seen as matching if its distance to the field post is smaller than maxConst.
   */
  const float maxConst = 0.5f;
  float minDistance = maxConst;
  for (auto& fieldGoalPost : fieldInfo_.goalPosts)
  {
    float d = (goalPost - fieldGoalPost).norm();
    if (d < minDistance)
    {
      minDistance = d;
    }
  }
  return maxConst / (maxConst + 2 * minDistance);
}
