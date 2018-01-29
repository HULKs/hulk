#pragma once

#include <Tools/Math/Eigen.hpp>
#include <Tools/Time.hpp>

#include <Framework/Module.hpp>

#include "Data/BallData.hpp"
#include "Data/BallState.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/OdometryOffset.hpp"
#include "Data/PlayerConfiguration.hpp"

class Brain;

class BallFilter : public Module<BallFilter, Brain>
{
public:
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
    Vector2f x;
    /// covariance matrix if resting
    Matrix2f covX;
    /// the filtered score of the resting equivalent
    float error = 1.f;
  };
  struct MovingEquivalent
  {
    /// the current filtered position of the ball (meters)
    Vector2f x;
    /// the current filtered velocity of the ball (meters per second)
    Vector2f dx;
    /// covariance matrix of the position of the ball
    Matrix2f covX;
    /// cross covariance matrix of the velocity and position of the ball
    Matrix2f covDxX;
    /// covariance matrix of the velocity of the ball
    Matrix2f covDx;
    /// the filtered score of the moving equivalent
    float error = 1.f;
  };
  struct BallMode
  {
    /// true if ball ist assumed to be resting
    bool resting = false;
    /// the equivalent hypothesis if this ball was resting
    RestingEquivalent restingEquivalent;
    /// the equivalent hypothesis if this ball was moving
    MovingEquivalent movingEquivalent;

    /// the number of measurements that have been evaluated since the filter has been started
    /// max. 33 balls per seconds => the overflow occurs after 4.1 years
    unsigned int measurements;
    /// timestamp of the last ball update
    TimePoint lastUpdate;
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
   * @brief measurement the relative position of the observation by vision
   */
  void update(const Vector2f& measurement);
  /**
   * @brief updateMovingEquivalent updates the moving ball hypothesis of a mode
   * @param movingEquivalent a reference to the moving equivalent
   * @param measurement a reference to the measurement
   */
  void updateMovingEquivalent(MovingEquivalent& movingEquivalent, const Vector2f& measurement);
  /**
   * @brief updateRestingEquivalent updates the resting ball hypothesis of a mode
   * @param restingEquivalent a reference to the resting equivalent
   * @param measurement a reference to the measurement
   */
  void updateRestingEquivalent(RestingEquivalent& restingEquivalent, const Vector2f& measurement);

  /**
   * @brief selectBestMode finds out which of the modes could be the real ball
   */
  void selectBestMode();
  /// process covariance matrix of the position
  const Parameter<Matrix2f> processCovX_;
  /// process cross covariance matrix of the velocity and position
  const Parameter<Matrix2f> processCovDxX_;
  /// process covariance matrix of the velocity
  const Parameter<Matrix2f> processCovDx_;
  /// measurement covariance matrix
  const Parameter<Matrix2f> measurementCov_;
  /// the maximal distance in meters that a measurement may be away from a mode to be merged into it
  const Parameter<float> maxAssociationDistance_;
  /// friction parameter to model linear friction of type Fr = mu * N
  const Parameter<float> ballFrictionMu_;
  /// the hysteresis applied to classify a ball as moving, when resting before
  const Parameter<float> movingHysteresis_;
  /// the PlayerConfiguration
  const Dependency<PlayerConfiguration> playerConfiguration_;
  /// the ball data from vision
  const Dependency<BallData> ballData_;
  /// the dimensions of the field
  const Dependency<FieldDimensions> fieldDimensions_;
  /// a reference to the odometry offset
  const Dependency<OdometryOffset> odometryOffset_;
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
  /// time point of the last prediction
  TimePoint lastPrediction_;
  /// the time when the ball was lost
  TimePoint timeWhenBallLost_;
  /// the timestamp of the last ball data
  TimePoint lastTimestamp_;
};
