#include <cmath>

#include "Modules/NaoProvider.h"
#include "Tools/Chronometer.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Line.hpp"
#include "Tools/Math/Random.hpp"

#include "PositionKnowledge.hpp"


PositionKnowledge::PositionKnowledge(const ModuleManagerInterface& manager)
  : Module(manager, "PositionKnowledge")
  , sigma_(*this, "sigma", [] {})
  , sigmaPrediction_(*this, "sigmaPrediction", [] {})
  , numberOfParticles_(*this, "numberOfParticles", [] {})
  , lineAssociationDistance_(*this, "lineAssociationDistance", [] {})
  , lineAssociationAngle_(*this, "lineAssociationAngle", [this] { lineAssociationAngle_() *= TO_RAD; })
  , maxGyroNormWhenMeasuring_(*this, "maxGyroNormWhenMeasuring", [] {})
  , maxProjectionDistance_(*this, "maxProjectionDistance", [] {})
  , maxProjectionDistanceLowNoise_(*this, "maxProjectionDistanceLowNoise", [] {})
  , sensorResettingFraction_(*this, "sensorResettingFraction", [] {})
  , transmitParticles_(*this, "transmitParticles", [] {})
  , cycleInfo_(*this)
  , lineData_(*this)
  , goalData_(*this)
  , gameControllerState_(*this)
  , playerConfiguration_(*this)
  , landmarkModel_(*this)
  , bodyPose_(*this)
  , motionState_(*this)
  , odometryOffset_(*this)
  , fieldDimensions_(*this)
  , imuSensorData_(*this)
  , imageData_(*this)
  , jointSensorData_(*this)
  , robotPosition_(*this)
  , fieldInfo_(*playerConfiguration_, *fieldDimensions_)
  , positionProvider_(*this, fieldInfo_, *gameControllerState_, *playerConfiguration_, *landmarkModel_, *fieldDimensions_)
  , particles_()
  , pose_()
  , lastPose_()
  , lastTimeJumped_()
  , lastState_(GameState::INITIAL)
  , lastPenalty_(Penalty::NONE)
  , currentMaxProjectionDistance_(maxProjectionDistance_())
  , useMeasurements_(false)
  , updatedWithMeasurements_(false)
  , resettedThisCycle_(false)
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

void PositionKnowledge::cycle()
{
  Chronometer time(debug(), mount_ + ".cycle_time");
  resettedThisCycle_ = false;
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
  if (resettedThisCycle_)
  {
    robotPosition_->valid = false;
  }
  else
  {
    robotPosition_->valid = true;
  }
  debug().update(mount_, *this);
}

void PositionKnowledge::updateState()
{
  if (gameControllerState_->secondary == SecondaryState::PENALTYSHOOT)
  {
    // Penalty Shootout requires special handling. The robot is in SET when waiting.
    // It will be switched to PLAYING when the robot should start to move.
    if ((gameControllerState_->state == GameState::PLAYING && lastState_ != GameState::PLAYING) ||
        (gameControllerState_->penalty == Penalty::NONE && lastPenalty_ != Penalty::NONE))
    {
      // All particles are replaced with particles that correspond to the positions according to the rules.
      for (auto& particle : particles_)
      {
        // There is only one possible state in a penalty shootout, thus all particles get the same cluster ID.
        particle.replace(positionProvider_.getPenaltyShootout(), 0);
      }
      nextClusterID_ = 1;
      resetWeights();
      resettedThisCycle_ = true;
    }
  }
  else
  {
    if (gameControllerState_->penalty == Penalty::NONE && lastPenalty_ != Penalty::NONE && lastPenalty_ != Penalty::ILLEGAL_MOTION_IN_SET)
    {
      if (gameControllerState_->state == GameState::SET)
      {
        // Robots that are unpenalized during SET are manually placed.
        unsigned int clusterID;
        for (auto& particle : particles_)
        {
          // There are two penalized positions, and since we don't know exactly which is returned, each particle gets its own cluster.
          Pose pose(positionProvider_.getManuallyPlaced(clusterID));
          particle.replace(pose, clusterID);
        }
        nextClusterID_ = 5;
      }
      else
      {
        // All particles are being replaced by particles near the penalize position as written in the config.
        unsigned int clusterID;
        for (auto& particle : particles_)
        {
          // There are two penalized positions, and since we don't know exactly which is returned, each particle gets its own cluster.
          Pose pose(positionProvider_.getPenalized(clusterID));
          particle.replace(pose, clusterID);
        }
        nextClusterID_ = 2;
      }
      resetWeights();
      resettedThisCycle_ = true;
    }
    else if ((gameControllerState_->state == GameState::INITIAL && lastState_ != GameState::INITIAL) ||
             (gameControllerState_->state == GameState::READY && lastState_ == GameState::INITIAL))
    {
      // All particles are being replaced by particles near the initial position of
      // the given player number (as in config). Robot position is not valid(ated) in this state.
      unsigned int clusterID;
      for (auto& particle : particles_)
      {
        // There is currently only one initial position, so each particle gets the same cluster ID.
        Pose pose(positionProvider_.getInitial(clusterID));
        particle.replace(pose, clusterID);
      }
      nextClusterID_ = 2; // There are at most 2 initial clusters.
      resetWeights();
      resettedThisCycle_ = true;
    }
    else if (gameControllerState_->state == GameState::PLAYING && lastState_ == GameState::SET)
    {
      // If the gamestate changed (after it has changed!) from SET to PLAYING
      if (wasHighInSet_)
      {
        unsigned int clusterID;
        for (auto& particle : particles_)
        {
          Pose pose(positionProvider_.getManuallyPlaced(clusterID));
          particle.replace(pose, clusterID);
        }
        nextClusterID_ = 5;
        resetWeights();
        resettedThisCycle_ = true;
        wasHighInSet_ = false;
      }
      else
      {
        unsigned int clusterID;
        for (auto& particle : particles_)
        {
          if (particle.pose.position.x() > 0.f ||
              (!gameControllerState_->kickoff && particle.pose.position.norm() < (fieldDimensions_->fieldCenterCircleDiameter * 0.5f - 0.2f)))
          {
            // Particle should be replaced. It is not possible to start a game
            // in the opponent's half or outside the field.
            // There are generally multiple manual placement positions, so each particle gets its own cluster ID.
            Pose pose(positionProvider_.getManuallyPlaced(clusterID));
            particle.replace(pose, nextClusterID_ + clusterID);
            particle.weight = 1.f / numberOfParticles_();
          }
        }
        nextClusterID_ += 5;
      }
    }
    else if (gameControllerState_->state == GameState::SET)
    {
      if ((!bodyPose_->footContact || wasHighInSet_) && (motionState_->bodyMotion == MotionRequest::BodyMotion::STAND))
      {
        wasHighInSet_ = true;
        unsigned int clusterID;
        for (auto& particle : particles_)
        {
          Pose pose(positionProvider_.getManuallyPlaced(clusterID));
          particle.replace(pose, clusterID);
        }
        nextClusterID_ = 5;
        resetWeights();
        resettedThisCycle_ = true;
      }
    }
  }

  if (gameControllerState_->penalty != Penalty::NONE || gameControllerState_->secondary == SecondaryState::PENALTYSHOOT)
  {
    useMeasurements_ = false;
    // Only localize while walking or standing
  }
  else if ((motionState_->bodyMotion == MotionRequest::BodyMotion::WALK || motionState_->bodyMotion == MotionRequest::BodyMotion::STAND) &&
           imuSensorData_->gyroscope.norm() < maxGyroNormWhenMeasuring_())
  {
    useMeasurements_ = true;
  }
  else
  {
    useMeasurements_ = false;
  }

  lastState_ = gameControllerState_->state;
  lastPenalty_ = gameControllerState_->penalty;
}

void PositionKnowledge::integrateOdometry()
{
  const Pose& odometry = odometryOffset_->odometryOffset;
  const auto distanceMoved = Vector2f(std::abs(odometry.position.x()), std::abs(odometry.position.y()));
  const auto distanceRotated = std::abs(odometry.orientation);
  const float constantFactor = 0.00001;

  Vector3f sigma(constantFactor + distanceMoved.x() * sigmaPrediction_().x(), constantFactor + distanceMoved.y() * sigmaPrediction_().y(),
                 constantFactor + distanceRotated * sigmaPrediction_().z());
  for (auto& particle : particles_)
  {
    // Add noise to model the inaccuracies of the odometry.
    Pose noisyOffset = positionProvider_.addGaussianNoise(odometry, sigma);
    predictParticleMovement(particle, noisyOffset);
  }
}

void PositionKnowledge::integrateMeasurements()
{
  if (lineData_->timestamp != lastLineTimestamp_ && !lineData_->edges.empty())
  {
    lastLineTimestamp_ = lineData_->timestamp;
    // LineData contains a graph made of vertices and edges originally inteded to represent corners or T crossings.
    // The original lines can be obtained like this:
    std::vector<Line<float>> lines;
    for (auto& it : lineData_->edges)
    {
      lines.emplace_back(lineData_->vertices[it.x()], lineData_->vertices[it.y()]);
    }
    int measurementCount = 0;
    const int maxMeasurementCount = 10;
    // Prune the lines before updating the particles:
    for (auto& line : lines)
    {
      if (measurementCount > maxMeasurementCount)
      {
        break;
      }
      // TODO: Make this configurable
      if ((line.p1 - line.p2).norm() < 0.15f)
      {
        continue;
      }
      float projectionDistance = distPointToLineSegment(line, {0.f, 0.f});
      // TODO: maybe also constraint this to set
      const bool lowBodyTiltUncertainty = motionState_->bodyMotion == MotionRequest::BodyMotion::STAND;
      currentMaxProjectionDistance_ = lowBodyTiltUncertainty ? maxProjectionDistanceLowNoise_() : maxProjectionDistance_();
      if (projectionDistance > currentMaxProjectionDistance_)
      {
        // lines that are projected over a large distance are fully dropped
        continue;
      }
      measurementCount++;
      // if none of the lines passed the checks, updatedWithMeasurements_ will stay false
      updatedWithMeasurements_ = true;

      // update each line
      for (auto& particle : particles_)
      {
        updateWithLine(particle, line, projectionDistance);
      }
    }
    updatedWithMeasurements_ = true;
  }
}

void PositionKnowledge::resample()
{
  std::vector<PositionParticle> savedParticles = particles_;
  // Normalize particle weights.
  float weightSum = 0;
  for (auto& it : savedParticles)
  {
    weightSum += it.weight;
  }
  debug().update("PositionKnowledge.weightSum", weightSum);
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
      particle.replace(positionProvider_.addGaussianNoise(pose_, {sigma_().x() * 10.f, sigma_().y() * 10.f, sigma_().z()}), nextClusterID_++);
    }

    resetWeights();
    resettedThisCycle_ = true;
    return;
  }
  for (auto& it : savedParticles)
  {
    it.weight /= weightSum;
  }
  // The keeper is not allowed to use resetting at all
  bool useSensorResetting = (positionProvider_.isSensorResettingAvailable() && playerConfiguration_->playerNumber != 1);
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
      particles_.emplace_back(positionProvider_.addGaussianNoise(oldParticle.pose, sigma_()), oldParticle.clusterID);
      currentSum += p;
    }
  }
  if (useSensorResetting)
  {
    // calculate score to chose resetting postion:
    float maxPositionDiff = fieldDimensions_->fieldLength * 0.5f + fieldDimensions_->fieldBorderStripWidth;

    Pose resettingPose = positionProvider_.getSensorResetting();
    Pose mirroredResettingPose = {-resettingPose.position, static_cast<float>(resettingPose.orientation + M_PI)};

    float resettingPositionDiff = (resettingPose.position - mirroredResettingPose.position).squaredNorm();
    const float minResettingPositionDiff = fieldDimensions_->fieldLength * fieldDimensions_->fieldLength * 0.0625;

    float resettingScore = 0.f;
    float mirroredResettingScore = 0.f;

    const float angleDiff = Angle::angleDiff(pose_.orientation, resettingPose.orientation);
    const float angleScore = 1.f - angleDiff / M_PI;

    const float positionDiff = (pose_.position - resettingPose.position).norm();
    const float positionScore = 1.f - positionDiff / maxPositionDiff;

    const float mirroredAngleDiff = Angle::angleDiff(pose_.orientation, mirroredResettingPose.orientation);
    const float mirroredAngleScore = 1.f - mirroredAngleDiff / M_PI;

    const float mirroredPositionDiff = (pose_.position - resettingPose.position).norm();
    const float mirroredPositionScore = 1.f - mirroredPositionDiff / maxPositionDiff;

    // Check whether the resetting positions are sufficiently differ in position (not close to center)
    if (resettingPositionDiff < minResettingPositionDiff)
    {
      // if the restting pose is to close to the center, only reset by orientation (relying on the imu sensor fusion)
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
        particles_.emplace_back(positionProvider_.addGaussianNoise(resettingPose, sigma_()), nextClusterID_++);
      }
    }
    else
    {
      for (unsigned int i = 0; i < numberOfParticles_() - keepParticles; i++)
      {
        particles_.emplace_back(positionProvider_.addGaussianNoise(mirroredResettingPose, sigma_()), nextClusterID_++);
      }
    }
  }
  // TODO: Maybe merge clusters.
}

void PositionKnowledge::computePosition()
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
      // Since the mean of an angle of 359deg and 1deg should be 0deg and not 180deg, weighted direction vectors are summed.
      direction += Vector2f(std::cos(particle.pose.orientation), std::sin(particle.pose.orientation)) * particle.weight;
    }
  }
  position /= bestWeightSum;
  direction /= bestWeightSum;
  pose_ = Pose(position, atan2(direction.y(), direction.x()));
}

void PositionKnowledge::resetWeights()
{
  // Set all particle weights to an equal value.
  float w = 1.f / numberOfParticles_();
  for (auto& it : particles_)
  {
    it.weight = w;
  }
}

void PositionKnowledge::evaluateParticle()
{
  // Positions outside the carpet should be impossible. One could think of replacing the sample (sensor resetting) instead of setting its weight to 0.
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
    if (playerConfiguration_->playerNumber == 1 && gameControllerState_->secondary != SecondaryState::PENALTYSHOOT)
    {
      if (particle.pose.position.x() > 0)
      {
        particle.weight = 0;
      }
    }
  }
}

void PositionKnowledge::updateLastTimeJumped()
{
  // Calculate the last time jumped
  const float jumpDistThreshSquared = 0.5f * 0.5f;
  const float jumpAngleThresh = 30 * TO_RAD;
  if ((pose_.position - lastPose_.position).squaredNorm() > jumpDistThreshSquared ||
      Angle::angleDiff(pose_.orientation, lastPose_.orientation) > jumpAngleThresh)
  {
    lastTimeJumped_ = cycleInfo_->startTime;
  }
  lastPose_ = pose_;
}

void PositionKnowledge::toValue(Uni::Value& value) const
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

void PositionKnowledge::predictParticleMovement(PositionParticle& particle, const Pose& pose)
{
  particle.pose += pose;
}

void PositionKnowledge::updateWithLine(PositionParticle& particle, const Line<float>& line, const float projectionDistance)
{
  // TODO: Actually this should be the outer loop (iterate over the particles in the inner loop)
  Line<float> worldLine(particle.pose * line.p1, particle.pose * line.p2);
  particle.weight *= weightByLine(worldLine, projectionDistance);
}

void PositionKnowledge::updateWithLandMarkPosition(PositionParticle& particle, const Vector2f& relativeLandmarkMeasurement, const float measurementVariance,
                                                   const Vector2f& absoluteGroundTruthPosition)
{
  const Vector2f relativeGroundTruthPosition = particle.pose.inverse() * absoluteGroundTruthPosition;
  const float squaredError = (relativeLandmarkMeasurement - relativeGroundTruthPosition).squaredNorm();
  const float weightByFieldMarkMeasurement = std::exp(-0.5f * squaredError / measurementVariance);

  particle.weight *= weightByFieldMarkMeasurement;
}

void PositionKnowledge::updateWithGoalPosts(PositionParticle& particle, const VecVector2f& goalPosts)
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

float PositionKnowledge::angleBetweenLineVectors(const Vector2f& v1, const Vector2f& v2) const
{
  const float a1 = std::atan2(v1.y(), v1.x());
  const float a2 = std::atan2(v2.y(), v2.x());

  float phi = std::fmod(std::abs(a1 - a2), M_PI);
  phi = phi > M_PI_2 ? M_PI - phi : phi;
  return phi;
}

float PositionKnowledge::distPointToLineSegment(const Line<float>& line, const Vector2f& point) const
{
  // Return minimum distance between line segment "line" and point "point"
  const float l2 = (line.p2 - line.p1).squaredNorm();
  if (l2 == 0.0)
  {
    return (point - line.p1).norm();
  }

  // Consider the line extending the segment, parameterized as p1 + t * (p2 - p1).
  // We find projection of point "point" onto the line.
  // It falls where t = [(p - p1) . (p2 - p1)] / |p2 - p1|^2

  const float t = (point - line.p1).dot(line.p2 - line.p1) / l2;

  if (t < 0.0)
  {
    return (point - line.p1).norm();
  }
  else if (t > 1.0)
  {
    return (point - line.p2).norm();
  }
  const Vector2f projection = line.p1 + (line.p2 - line.p1) * t;

  return (point - projection).norm();
}

float PositionKnowledge::weightByLine(const Line<float>& line, const float projectionDistance) const
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
    float distCenter = distPointToLineSegment(fieldLine, lineCenterPoint);
    // Check if line was associated with center circle
    bool associatedWithCircle = (fieldLine.p1.x() != fieldLine.p2.x()) && (fieldLine.p1.y() != fieldLine.p2.y());
    float lineAngleDiff = angleBetweenLineVectors(lineVector, fieldLineVector);
    // Drop lines that are far off
    if (distCenter > lineAssociationDistance_() || ((lineAngleDiff > lineAssociationAngle_() || lineLength > 1.2f * fieldLineLength) && !associatedWithCircle))
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
  const float projectionWeight = 1 - projectionDistance / currentMaxProjectionDistance_;
  // if the projection distance is the projectionWeight drops to 0. Thus the error of a far away line has a low impact on the total weight
  return std::pow(weight, projectionWeight);
}

float PositionKnowledge::weightByGoalPost(const Vector2f& goalPost) const
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
