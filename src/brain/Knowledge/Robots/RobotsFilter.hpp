#pragma once

#include "Data/BodyPose.hpp"
#include "Data/CameraMatrix.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/FilteredRobots.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/RobotData.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/RobotProjection.hpp"
#include "Framework/Module.hpp"
#include "Tools/StateEstimation/BhattacharyyaDistance.hpp"
#include "Tools/StateEstimation/MahalanobisDistance.hpp"
#include "Tools/StateEstimation/ProjectionMeasurementModel.hpp"


class Brain;

/**
 * @brief RobotsFilter
 */
class RobotsFilter : public Module<RobotsFilter, Brain>
{
public:
  struct Robot : public Uni::To
  {
    Robot(Vector4f initialState, Matrix4f initialStateCovariance)
      : filter(initialState, initialStateCovariance, Matrix4f::Identity(), Vector4f::Zero(),
               Matrix4f::Identity())
      , lastUpdate(TimePoint::getCurrentTime())
    {
    }
    KalmanFilter<4, 1> filter;
    /// the number of measurements since the filter has been started
    unsigned int measurements{1};
    /// timestamp of the last robot update
    TimePoint lastUpdate;
    Vector2f getPosition() const
    {
      return Vector2f(filter.getState().x(), filter.getState().y());
    }
    Matrix2f getPositionCovariance() const
    {
      return filter.getCovariance().block(0, 0, 2, 2);
    }
    Vector2f getVelocity() const
    {
      return Vector2f(filter.getState().z(), filter.getState().w());
    }
    void update(const Vector2f measurement, const Matrix2f measurementCovariance)
    {
      Eigen::Matrix<float, 2, 4> stateToMeasurementMatrix;
      stateToMeasurementMatrix << Matrix2f::Identity(), Matrix2f::Zero();
      filter.update(measurement, measurementCovariance, stateToMeasurementMatrix);
      lastUpdate = TimePoint::getCurrentTime();
      measurements++;
    }
    void merge(const Robot& otherRobot)
    {
      filter.update(otherRobot.filter.getState(), otherRobot.filter.getCovariance());
      // measurement counter is not incremented as well as lastUpdate is not set
    }
    bool isMergable(const Robot& otherRobot, const float maxPositionDistance,
                   const float euclideanThreshold, const float bhattacharyyaThreshold) const
    {
      const float dist = (otherRobot.getPosition() - getPosition()).norm();
      if (dist > maxPositionDistance)
      {
        return false;
      }
      if (dist < euclideanThreshold)
      {
        return true;
      }
      const auto bat = BhattacharyyaDistance::bhattacharyyaDistance<4>(
          otherRobot.filter.getState(), otherRobot.filter.getCovariance(), filter.getState(),
          filter.getCovariance());
      return bat < bhattacharyyaThreshold;
    }

    void toValue(Uni::Value& value) const override
    {
      value = Uni::Value(Uni::ValueType::OBJECT);
      value["state"] << filter.getState();
      value["covariance"] << filter.getCovariance();
      value["measurements"] << measurements;
      value["lastUpdate"] << lastUpdate;
    }
  };

  /// the name of this module
  ModuleName name = "RobotsFilter";
  /**
   *@brief The constructor of this class
   */
  RobotsFilter(const ModuleManagerInterface& manager);

  void cycle() override;


private:
  const Dependency<BodyPose> bodyPose_;
  const Dependency<CameraMatrix> cameraMatrix_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<RobotData> robotData_;
  const Dependency<RobotPosition> robotPosition_;

  /// initial state covariance of the diagonal velocity elements
  const Parameter<float> initialVelocityVariance_;
  /// factor to control velocity variance in (m / s)^2
  const Parameter<float> velocityVariance_;
  /// maximum radius of a measurement be associated with robot
  const Parameter<float> associateThreshold_;
  /// minimum measurements a robot has to have to be exposed to production
  const Parameter<unsigned int> minMeasurements_;
  /// maximum time a robot can have no measurements to be exposed to production
  const Parameter<float> maxTimeSinceLastUpdate_;
  /// maximum radius of robots being merged
  const Parameter<float> mergeRadius_;
  /// the bhattacharyyaThreshold when comparing to hypotheses
  const Parameter<float> mergeSimilarityThreshold_;
  /// the base variance of measurements (added to every error propagation)
  Parameter<Vector2f> measurementBaseVariance_;
  /// the basic deviation of the camera matrix roll, pitch and yaw in deg
  Parameter<Vector3f> cameraRPYDeviation_;
  /// the maximum distance to a measurement to be associated
  const Parameter<float> maxDistanceToMeasurement_;
  /// the time a robot is tracked without any measurements
  const Parameter<float> timeKeepRobotInFilter_;
  /// the maximum distance a robot can be predicted without a measurement to be exposed
  const Parameter<float> maxDistancePredicted_;

  /// the measurement model to estimate point covariances
  ProjectionMeasurementModel projectionMeasurementModel_;
  /// all robots currently tracked by a filter
  std::list<Robot> trackedRobots_;

  /// the Production of this module
  Production<FilteredRobots> filteredRobots_;

  /*
   * @brief Remove all robots being old from the tracked robots
   */
  void removeOldRobots();
  /*
   * @brief predict the next state of each robot currently tracked
   */
  void predictRobots();
  /*
   * @brief associate all measurements with tracked robots, creates a new robot if it cannot be
   * associated
   */
  void processMeasurements();
  /*
   * @brief merge robots close to each other
   */
  void mergeRobots();
  /*
   * @brief produces the filteredRobots_
   */
  void publishFilteredRobots();
  /*
   * @brief send debug data
   */
  void sendDebug() const;
};
