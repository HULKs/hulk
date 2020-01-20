#pragma once

#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/MovingAverage.hpp"
#include "Tools/StateEstimation/ProjectionMeasurementModel.hpp"
#include "Tools/Time.hpp"
#include <deque>

#include "Framework/Module.hpp"

#include "Data/BallData.hpp"
#include "Data/BallState.hpp"
#include "Data/CameraMatrix.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/OdometryOffset.hpp"
#include "Data/PlayerConfiguration.hpp"

class Brain;

class BallFilter : public Module<BallFilter, Brain>
{
public:
  /// the name of this module
  ModuleName name = "BallFilter";
  /**
   * @brief BallFilter initializes filter values and the state
   * @param manager reference to brain
   */
  BallFilter(const ModuleManagerInterface& manager);
  /**
   * @brief cycle does the filtering
   */
  void cycle();

private:
  struct RestingEquivalent
  {
    /// the current state if resting
    Vector2f x = Vector2f::Zero();
    /// covariance matrix if resting
    Matrix2f covX = Matrix2f::Identity();
    /// the filtered score of the resting equivalent
    float error = 1.f;
  };
  struct MovingEquivalent
  {
    /// the current filtered position of the ball (meters)
    Vector2f x = Vector2f::Zero();
    /// the current filtered velocity of the ball (meters per second)
    Vector2f dx = Vector2f::Zero();
    /// covariance matrix of the position of the ball
    Matrix2f covX = Matrix2f::Identity();
    /// cross covariance matrix of the velocity and position of the ball
    Matrix2f covDxX = Matrix2f::Identity();
    /// covariance matrix of the velocity of the ball
    Matrix2f covDx = Matrix2f::Identity();
    /// the filtered score of the moving equivalent
    float error = 1.f;
  };
  struct BallMode
  {
    /**
     * @brief BallMode constructor
     * @param ballFilter a reference to the ball filter (this module)
     *
     * Initialize measurementBuffer with size that covers one second of measurements.
     */
    BallMode(const BallFilter& ballFilter)
      : maxBufferSize(static_cast<size_t>(1.0f / ballFilter.cycleInfo_->cycleTime))
    {
    }

    float getPerceptsPerSecond(const float cycleTime) const
    {
      const auto currentSize = validityBuffer.size();
      assert(currentSize <= maxBufferSize);
      assert(currentSize > 0);
      int numValidPercepts = 0;

      for (const auto& v : validityBuffer)
      {
        if (v > 0.f)
        {
          numValidPercepts++;
        }
      }
      return numValidPercepts / (currentSize * cycleTime);
    }

    void resetValidityBuffer()
    {
      validityBuffer.assign(maxBufferSize, 0.f);
    }

    float getMeanValidity() const
    {
#ifndef NDEBUG
      const auto currentSize = validityBuffer.size();
      assert(currentSize <= maxBufferSize);
      assert(currentSize > 0);
#endif
      float sum = 0;
      int validPercepts = 0;
      for (const auto& v : validityBuffer)
      {
        if (v > 0.f)
        {
          sum += v;
          validPercepts++;
        }
      }
      return validPercepts > 0 ? (sum / validPercepts) : 0.f;
    }

    void addPerceptValidity(float validity)
    {
      validityBuffer.push_front(validity);
      if (validityBuffer.size() > maxBufferSize)
      {
        validityBuffer.pop_back();
      }
      assert(validityBuffer.size() <= maxBufferSize);
    }

    /// true if ball ist assumed to be resting
    bool resting = false;
    /// the equivalent hypothesis if this ball was resting
    RestingEquivalent restingEquivalent;
    /// the equivalent hypothesis if this ball was moving
    MovingEquivalent movingEquivalent;

    /// the number of measurements that have been evaluated since the filter has been started
    /// max. 33 balls per seconds => the overflow occurs after 4.1 years
    unsigned int measurements;
    /// circular buffer of measurements of the past second
    std::deque<float> validityBuffer;
    /// maximal buffer size of the previous validities
    size_t maxBufferSize;

    /// timestamp of the last ball update
    TimePoint lastUpdate;
    /// the validity estimate filtered for slow decay and fast increase
    float filteredValidity = 0.f;
  };
  /**
   * @brief predictBallDestination predicts the ball destination
   * @param ballMode the ball mode to calculate the destination for
   * @return the predicted ball destination
   */
  Vector2f predictBallDestination(const BallMode& ballMode) const;
  /**
   * @brief predict integrates odometry updates into the relative ball position
   */
  void predict();
  /**
   * @brief update integrates the ball measurement into a ball mode
   * @param measurement the relative position of the observation by vision
   * @param the validity of the measurement used for the update. [0, 1]
   */
  void update(const Vector2f& measurement, const float perceptValidity);
  /**
   * @brief updateMovingEquivalent updates the moving ball hypothesis of a mode
   * @param movingEquivalent a reference to the moving equivalent
   * @param measurementMean the mean of the measurement (relative position of the ball as detected
   * by the ball detection)
   * @param measurementCov the covariance of the measurement as obtained from the measurement model
   */
  void updateMovingEquivalent(MovingEquivalent& movingEquivalent, const Vector2f& measurementMean,
                              const Matrix2f& measurementCov);
  /**
   * @brief updateRestingEquivalent updates the resting ball hypothesis of a mode
   * @param restingEquivalent a reference to the resting equivalent
   * @param measurementMean the mean of the measurement (relative position of the ball as detected
   * by the ball detection)
   * @param measurementCov the covariance of the measurement as obtained from the measurement model
   */
  void updateRestingEquivalent(RestingEquivalent& restingEquivalent,
                               const Vector2f& measurementMean, const Matrix2f& measurementCov);
  /**
   * @brief sendDebug send all the debug data via the debug protocoll
   */
  void sendDebug() const;

  /**
   * @brief selectBestMode finds out which of the modes could be the real ball
   */
  void selectBestMode();

  /**
   * @brief calculate the validity for each mode in ballModes_
   * (Same as Nao Devils)
   */
  void updateValidities();

  /**
   * @brief check if we are in penalty keeper mode
   * @return true for penalty keeper
   */
  bool isPenaltyKeeper() const;

  /// process covariance of the position resting equivalent
  const Parameter<Matrix2f> restingProcessCovX_;
  /// process covariance matrix of the position for the moving equivalent
  const Parameter<Matrix2f> movingProcessCovX_;
  /// process cross covariance matrix of the velocity and position for the moving equivalent
  const Parameter<Matrix2f> movingProcessCovDxX_;
  /// process covariance matrix of the velocity  for the moving equivalent
  const Parameter<Matrix2f> movingProcessCovDx_;
  /// the base variance of measurements (added to every error propagation)
  Parameter<Vector2f> measurementBaseVariance_;
  /// the basic deviation of the camera matrix roll poitch an yaw in deg
  Parameter<Vector3f> cameraRPYDeviation_;
  /// the maximal distance in meters that a measurement may be away from a mode to be merged into it
  const Parameter<float> maxAssociationDistance_;
  /// friction parameter to model linear friction of type Fr = mu * N
  const Parameter<float> ballFrictionMu_;
  /// the relative threshold to classify a ball as moving (for relative comparison of the filtered
  /// association error)
  const ConditionalParameter<float> relativeMovingThreshold_;
  /// the low pass gain for the resting error filter
  const Parameter<float> restingErrorLowPassAlpha_;
  /// the low pass gain for the moving error filter
  const Parameter<float> movingErrorLowPassAlpha_;
  /// the absolute threshold to classify a ball as moving (absolute threshold for the filtered
  /// association error)
  const ConditionalParameter<float> maxRestingError_;
  /// the number of decceleration steps that need to be left to consider a ball resting
  const ConditionalParameter<int> numOfRestingDeccelerationSteps_;
  /// the number of measurements needed for a ball in order to make it a confident ball
  const Parameter<unsigned int> confidentMeasurementThreshold_;

  /// alpha parameter for filtering validity (used for slow decay when ball not seen)
  const Parameter<float> validityLowpassAlpha_;
  /// the validity of a new mode (TODO move to CNN?)
  const Parameter<float> defaultPerceptValidity_;
  /// the ratio of percepts expected for a confidently perceived ball
  const Parameter<float> confidentPerceptionRatio_;
  /// the game controller state
  const Dependency<GameControllerState> gameControllerState_;
  /// the PlayerConfiguration
  const Dependency<PlayerConfiguration> playerConfiguration_;
  /// the ball data from vision
  const Dependency<BallData> ballData_;
  /// the dimensions of the field
  const Dependency<FieldDimensions> fieldDimensions_;
  /// a reference to the odometry offset
  const Dependency<OdometryOffset> odometryOffset_;
  /// a reference to the camera matrix used for projecion error estimation
  const Dependency<CameraMatrix> cameraMatrix_;
  /// the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// the deceleration of the ball due to friction in m/s²
  float frictionDeceleration_;
  /// the result of the state estimation
  Production<BallState> ballState_;
  /// list of ball modes of the last few seconds
  std::list<BallMode> ballModes_;
  /// the accepted ball mode (or end if none is accepted)
  std::list<BallMode>::iterator bestMode_;
  /// the measurement model to estimate point covariances
  ProjectionMeasurementModel projectionMeasurementModel_;
  /// time point of the last prediction
  TimePoint lastPrediction_;
  /// the time when the ball was lost
  TimePoint timeWhenBallLost_;
  /// the timestamp of the last ball data
  TimePoint lastTimestamp_;
};
