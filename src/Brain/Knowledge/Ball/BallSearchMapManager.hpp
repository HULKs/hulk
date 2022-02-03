#pragma once

#include "Framework/Module.hpp"

#include "Data/BallSearchMap.hpp"
#include "Data/BallState.hpp"
#include "Data/BodyPose.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/TeamPlayers.hpp"

class Brain;

class BallSearchMapManager : public Module<BallSearchMapManager, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"BallSearchMapManager"};

  /**
   * @brief BallSearchMapManager The constructor
   * @param manager A reference to the ModuleManagerInterface (e.g. brain)
   */
  explicit BallSearchMapManager(const ModuleManagerInterface& manager);

  void cycle() override;

private:
  /// A multiplier that is applied to any cell with a ball inside it (ball filter needs to be
  /// confident)
  Parameter<float> confidentBallMultiplier_;
  /// The core weight (x) of the convolution kernel that is applied to the field every cycle.
  /// [[1,1,1],[1,x,1],[1,1,1]]
  Parameter<int> convolutionKernelCoreWeight_;
  /// The field of view angle from the robot camera. Should be a bit smaller than the real angle.
  Parameter<float> fovAngle_;
  /// A ball that is older than the maxBallAge won't be recognized as a seen ball. Value given in
  /// seconds.
  Parameter<Clock::duration> maxBallAge_;
  /// The maximum distance the robot is able to see the ball really good.
  /// It is way worse to guess this value too big than too small!
  Parameter<float> maxBallDetectionRange_;
  /// Minimum threshold for the balls distance from the line to count as "out"
  Parameter<float> minBallOutDistance_;
  /// The minimum probability a cell should have after being upvoted (to prevent multiplication with
  /// 0)
  Parameter<float> minProbOnUpvote_;

  const Dependency<BallState> ballState_;
  const Dependency<BodyPose> bodyPose_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<JointSensorData> jointSensorData_;
  const Dependency<PlayerConfiguration> playerConfiguration_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<TeamPlayers> teamPlayers_;

  Production<BallSearchMap> ballSearchMap_;

  /// The own player data put into a teamPlayer struct.
  TeamPlayer ownPlayer_;
  /// All players on the field (includes own robot). May include penalized players.
  std::vector<const TeamPlayer*> allPlayers_;

  /// The maximum ball detection range (Already squared to save some operations per cycle).
  /// See corresponding parameter.
  float maxBallDetectionRangeSquared_;

  /// The field width given by the fieldDimensions (dependency)
  const float fieldWidth_;
  /// the field length given by the fieldDimensions (dependency)
  const float fieldLength_;

  /**
   * @brief Updates the map with all data available (all robot poses and ball data)
   */
  void updateMap();

  /**
   * @brief integrates the knowledge a player has to the map.
   * @param player The team player to integrate the knowledge from
   */
  void integrateRobotKnowledge(const TeamPlayer& player);

  /**
   * @brief Resets the whole map
   *
   * Age will be set to 0 for all cells. Probability will be set to 1/totalCellCount for all cells.
   */
  void resetMap();

  /**
   * @brief Initializes the map with a high probability around the center
   */
  void resetMapForReady();

  /**
   * @brief Distributes a given probability over a rectangular area.
   * @param p1 Start coordinates in (-1, -1) to (1, 1) coordinate space
   * @param p2 End coordinates in (-1, -1) to (1, 1) coordinate space
   * @param total Total probability distributed over the given area
   */
  void distributeProbability(const Vector2f& p1, const Vector2f& p2, const float totalProbability);

  /**
   * @brief Deletes all Probability cells and rebuilds the map.
   *
   * Mainly used for initialization of all fields in the production.
   */
  void rebuildProbabilityMap();
};
