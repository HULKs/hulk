#pragma once

#include <Data/FieldDimensions.hpp>
#include <Data/GameControllerState.hpp>
#include <Data/LandmarkModel.hpp>
#include <Data/PlayerConfiguration.hpp>
#include <Framework/Module.hpp>
#include <Tools/Math/Eigen.hpp>
#include <Tools/Math/Pose.hpp>

#include "FieldInfo.hpp"

class PositionProvider
{
public:
  /**
   * @brief PositionProvider initializes members and parameters
   * @param module the calling module (for parameters)
   * @param fieldInfo information about landmarks on the field
   * @param gameControllerState a reference to the game controller state
   * @param playerConfiguration a reference to the player configuration
   * @param landmarkModel a reference to the landmark model
   * @param fieldDimensions a reference to the field dimensions
   */
  PositionProvider(const ModuleBase& module, const FieldInfo& fieldInfo,
                   const GameControllerState& gameControllerState,
                   const PlayerConfiguration& playerConfiguration,
                   const LandmarkModel& landmarkModel, const FieldDimensions& fieldDimensions);
  /**
   * @brief resetHypothesesCounter resets the hypothesesCounter_
   */
  void resetHypothesesCounter() const;
  /**
   * @brief getOnField generates a random pose on the field
   * @return a pose on the field
   */
  Pose getOnField() const;
  /**
   * @brief getInitial generates a pose where the robot is placed in the INITIAL state
   * @param clusterHint a hint for the cluster ID to account for multimodal distributions
   * @param addNoise set to true to add gaussian noise
   * @return a pose for placement before the game
   */
  Pose getInitial(unsigned int& clusterHint, const bool addNoise = true) const;
  /**
   * @brief getPenalized generates a pose around one of the penalized poses
   * @param clusterHint a hint for the cluster ID to account for multimodal distributions
   * @param addNoise set to true to add gaussian noise
   * @return a pose when penalized
   */
  Pose getPenalized(unsigned int& clusterHint, const bool addNoise = true) const;
  /**
   * @brief getManuallyPlaced generates a pose around the manual placement poses
   * poses: http://www.tzi.de/spl/pub/Website/Downloads/Rules2016.pdf page 15.
   * @param clusterHint a hint for the cluster ID to account for multimodal distributions
   * @param addNoise set to true to add gaussian noise
   * @return a pose for manual placement
   */
  Pose getManuallyPlaced(unsigned int& clusterHint, const bool addNoise = true) const;
  /**
   * @brief getPenaltyShootout generates a pose where the robot is placed in a penalty shootout
   * @param clusterHint ID of the cluster the sampled position belongs to
   * @param multiPenaltyShootoutPositions true if there is more than one starting
   * position/orientation of shooter
   * @return a pose for penalty shootout
   */
  Pose getPenaltyShootout(unsigned int& clusterHint, const bool multiPenaltyShootoutPositions,
                          const bool addNoise = true) const;
  // TODO: only for testing
  bool circleWithOrientationAvailable() const;
  /**
   * @brief isSensorResettingAvailable indicates whether sensor resetting poses are available
   * @return true iff sensor resetting poses are available
   */
  bool isSensorResettingAvailable() const;
  /**
   * @brief getSensorResetting generates a pose from recent sensor readings
   * @return a pose that is generated from sensor readings
   */
  Pose getSensorResetting() const;
  /**
   * @brief addGaussianNoise adds noise to a pose
   * @param the original pose
   * @param standard deviation of the noise for each component
   * @return the pose with added noise
   */
  Pose addGaussianNoise(const Pose& pose, const Vector3f& standardDeviation) const;

private:
  /// standard deviation of the initial distribution
  const Parameter<Vector3f> sigmaInitial_;
  /// standard deviation of the penalized distribution
  const Parameter<Vector3f> sigmaPenalized_;
  /// instead of using a single hypothesis in INITIAL, a distribution over the whole sidelines in
  /// the own half is generated
  const Parameter<bool> startAnywhereAtSidelines_;
  /// the maximum number of hypothesis used for UKF-Localization
  const Parameter<unsigned int> maxNumberOfHypotheses_;
  /// information about landmarks on the field
  const FieldInfo& fieldInfo_;
  /// a reference to the game controller state
  const GameControllerState& gameControllerState_;
  /// a reference to the player configuration
  const PlayerConfiguration& playerConfiguration_;
  /// a reference to the landmark model
  const LandmarkModel& landmarkModel_;
  /// a reference to the field dimensions
  const FieldDimensions& fieldDimensions_;
  /// a counter for switching between multiple hypotheses (e.g. when penalized or manually placed).
  /// It does not matter whether it overflows.
  mutable unsigned int hypothesesCounter_;
};
