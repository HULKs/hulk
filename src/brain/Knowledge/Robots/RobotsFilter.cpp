#include "RobotsFilter.hpp"
#include "Tools/StateEstimation/UKF.hpp"
#include "print.h"


RobotsFilter::RobotsFilter(const ModuleManagerInterface& manager)
  : Module(manager)
  , bodyPose_(*this)
  , cameraMatrix_(*this)
  , cycleInfo_(*this)
  , fieldDimensions_(*this)
  , gameControllerState_(*this)
  , robotData_(*this)
  , robotPosition_(*this)
  , initialVelocityVariance_(*this, "initialVelocityVariance", [] {})
  , velocityVariance_(*this, "velocityVariance", [] {})
  , associateThreshold_(*this, "associateThreshold", [] {})
  , minMeasurements_(*this, "minMeasurements", [] {})
  , maxTimeSinceLastUpdate_(*this, "maxTimeSinceLastUpdate", [] {})
  , mergeRadius_(*this, "mergeRadius", [] {})
  , mergeSimilarityThreshold_(*this, "mergeSimilarityThreshold", [] {})
  , measurementBaseVariance_(*this, "measurementBaseVariance",
                             [this] {
                               projectionMeasurementModel_.resetParameters(
                                   cameraRPYDeviation_(), measurementBaseVariance_());
                             })
  , cameraRPYDeviation_(*this, "cameraRPYDeviation",
                        [this] {
                          cameraRPYDeviation_() *= TO_RAD;
                          projectionMeasurementModel_.resetParameters(cameraRPYDeviation_(),
                                                                      measurementBaseVariance_());
                        })
  , maxDistanceToMeasurement_(*this, "maxDistanceToMeasurement", [] {})
  , timeKeepRobotInFilter_(*this, "timeKeepRobotInFilter", [] {})
  , maxDistancePredicted_(*this, "maxDistancePredicted", [] {})
  , filteredRobots_(*this)
{
  cameraRPYDeviation_() *= TO_RAD;
  projectionMeasurementModel_ =
      ProjectionMeasurementModel(cameraRPYDeviation_(), measurementBaseVariance_());
}

void RobotsFilter::cycle()
{
  if (cycleInfo_->getTimeDiff(robotPosition_->lastTimeJumped) < 0.5f ||
      gameControllerState_->penalty != Penalty::NONE)
  {
    trackedRobots_.clear();
    return;
  }
  removeOldRobots();
  predictRobots();
  if (robotPosition_->valid)
  {
    processMeasurements();
  }
  mergeRobots();
  publishFilteredRobots();

  sendDebug();
}

void RobotsFilter::removeOldRobots()
{
  trackedRobots_.erase(std::remove_if(trackedRobots_.begin(), trackedRobots_.end(),
                                      [this](const Robot& robot) {
                                        // The more measurements there are for a robot, the longer
                                        // it is allowed to stay in the filter.
                                        return cycleInfo_->getTimeDiff(robot.lastUpdate) >
                                               (robot.measurements < timeKeepRobotInFilter_() * 2.f
                                                    ? static_cast<float>(robot.measurements) / 2.f
                                                    : timeKeepRobotInFilter_());
                                      }),
                       trackedRobots_.end());
}

void RobotsFilter::predictRobots()
{
  const float deltaT = cycleInfo_->cycleTime;
  Eigen::Matrix<float, 4, 4, Eigen::RowMajor> stateTransitionMatrix;
  stateTransitionMatrix << 1, 0, deltaT, 0, 0, 1, 0, deltaT, 0, 0, 1, 0, 0, 0, 0, 1;
  // This process covariance model assumes a constant acceleration error for the duration of a time
  // step. The maximum acceleration error is estimated by the velocityVariance_ parameter. A
  // constant acceleration error results in a position and velocity error proportional to E = [1/2 *
  // deltaT^2, deltaT] (for each of the two dimensions used here). The processCovariance is obtained
  // by E * E^T * velocityVariance_.
  Matrix4f processCovariance;
  processCovariance << Matrix2f::Ones() * std::pow(deltaT, 4) / 4.f,
      Matrix2f::Ones() * std::pow(deltaT, 3) / 2.f, Matrix2f::Ones() * std::pow(deltaT, 3) / 2.f,
      Matrix2f::Ones() * std::pow(deltaT, 2);
  processCovariance *= velocityVariance_();
  for (auto& robot : trackedRobots_)
  {
    robot.filter.setTransitionMatrix(stateTransitionMatrix);
    robot.filter.predict(processCovariance);
  }
}

void RobotsFilter::processMeasurements()
{
  if (bodyPose_->wonky || bodyPose_->fallen || !bodyPose_->footContact)
  {
    return;
  }
  struct FilteredMeasurement
  {
    FilteredMeasurement(Vector2f absPos, Matrix2f mesCov)
      : absolutePosition(std::move(absPos))
      , measurementCovariance(std::move(mesCov))
    {
    }
    const Vector2f absolutePosition{Vector2f::Zero()};
    const Matrix2f measurementCovariance{Matrix2f::Zero()};
    std::vector<RobotsFilter::Robot*> associatedRobots;
  };
  std::vector<FilteredMeasurement> filteredMeasurements;
  filteredMeasurements.reserve(robotData_->positions.size());
  for (const auto& measurement : robotData_->positions)
  {
    const float distRobotToMeasurement = measurement.norm();
    if (distRobotToMeasurement > maxDistanceToMeasurement_())
    {
      // reject measurements far away
      continue;
    }
    const Vector2f absoluteMeasurement = robotPosition_->robotToField(measurement);
    if (!fieldDimensions_->isInsideField(absoluteMeasurement, 0.5f))
    {
      // reject measurements outside the field
      continue;
    }
    const Matrix2f measurementCov = Rotation2Df(robotPosition_->pose.orientation).inverse() *
                                    projectionMeasurementModel_.computePointCovFromPositionFeature(
                                        measurement, cameraMatrix_->camera2ground) *
                                    Rotation2Df(robotPosition_->pose.orientation);
    filteredMeasurements.emplace_back(absoluteMeasurement, measurementCov);
  }

  for (auto& robot : trackedRobots_)
  {
    float minDistance{std::numeric_limits<float>::max()};
    FilteredMeasurement* closestMeasurement = nullptr;
    for (auto& measurement : filteredMeasurements)
    {
      const float distanceToMeasurement =
          (robot.getPosition() - measurement.absolutePosition).norm();
      if (distanceToMeasurement < associateThreshold_() && distanceToMeasurement < minDistance)
      {
        closestMeasurement = &measurement;
        minDistance = distanceToMeasurement;
      }
    }
    if (closestMeasurement != nullptr)
    {
      closestMeasurement->associatedRobots.emplace_back(&robot);
    }
  }
  for (const auto& measurement : filteredMeasurements)
  {
    if (measurement.associatedRobots.empty())
    {
      Matrix4f initialCov;
      initialCov << measurement.measurementCovariance, Matrix2f::Zero(), Matrix2f::Zero(),
          Matrix2f::Identity() * initialVelocityVariance_();
      trackedRobots_.emplace_back(
          Vector4f(measurement.absolutePosition.x(), measurement.absolutePosition.y(), 0, 0),
          initialCov);
      continue;
    }
    for (auto& associatedRobot : measurement.associatedRobots)
    {
      associatedRobot->update(measurement.absolutePosition, measurement.measurementCovariance);
    }
  }
}

void RobotsFilter::mergeRobots()
{
  for (auto itCurrent = trackedRobots_.begin(); itCurrent != trackedRobots_.end(); itCurrent++)
  {
    for (auto itCompare = std::next(itCurrent); itCompare != trackedRobots_.end();)
    {
      if (itCurrent->isMergable(*itCompare, mergeRadius_(), fieldDimensions_->robotDiameter / 2.f,
                                mergeSimilarityThreshold_()))
      {
        itCurrent->merge(*itCompare);
        itCompare = trackedRobots_.erase(itCompare);
      }
      else
      {
        itCompare++;
      }
    }
  }
}

void RobotsFilter::publishFilteredRobots()
{
  for (const auto& robot : trackedRobots_)
  {
    const float timeSinceLastUpdate = cycleInfo_->getTimeDiff(robot.lastUpdate);
    const float distancePredictedSinceLastUpdate = robot.getVelocity().norm() * timeSinceLastUpdate;
    if (robot.measurements > minMeasurements_() &&
        timeSinceLastUpdate < maxTimeSinceLastUpdate_() &&
        distancePredictedSinceLastUpdate < maxDistancePredicted_())
    {
      filteredRobots_->robots.emplace_back(robotPosition_->fieldToRobot(robot.getPosition()),
                                           robotPosition_->rotateFieldToRobot(robot.getVelocity()));
    }
  }
  filteredRobots_->valid = true;
}

void RobotsFilter::sendDebug() const
{
  debug().update(mount_ + "_robots", trackedRobots_);
}
