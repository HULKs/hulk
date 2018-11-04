#pragma once

#include "Data/BodyPose.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/ObstacleData.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/TeamObstacleData.hpp"
#include "Data/TeamPlayers.hpp"

#include "Framework/Module.hpp"
#include "Tools/Math/Eigen.hpp"


class Brain;

class TeamObstacleFilter : public Module<TeamObstacleFilter, Brain>
{
public:
  /// the name of this module
  ModuleName name = "TeamObstacleFilter";
  /**
   * @brief TeamObstacleFilter initializes members
   * @param manager a reference to brain
   */
  TeamObstacleFilter(const ModuleManagerInterface& manager);
  /**
   * @brief cycle updates the obstacles in the Map
   */
  void cycle();

private:
  /// flag to enable merging of unknown obstacles with other more specific obstacles
  Parameter<bool> reclassifyUnknownObstacles_;
  /// flag for using goal posts as obstacles
  Parameter<bool> goalPostsAreObstacles_;
  /// flag for using team players as obstacles (based on their localization)
  Parameter<bool> teamPlayersAreObstacles_;
  /// flag for using obstacles generated from the robot detection
  Parameter<bool> robotDetectionGeneratesObstacles_;
  /// flag for using obstacles generated from TeamPlayers obstacles
  Parameter<bool> useOtherRobotsObstacles_;
  /// the square of the radius up to which two obsacles can be merged
  Parameter<float> obstacleMergeRadiusSquared_;

  /// the estimation of the body pose. Used to classify whether this robot is fallen or not
  const Dependency<BodyPose> bodyPose_;
  /// the current gamestate as given by the game controller
  const Dependency<GameControllerState> gameControllerState_;
  /// a dependency on the obstacles of the ObstacleFilter
  const Dependency<ObstacleData> obstacleData_;
  /// a dependency on some information about the team mates
  const Dependency<TeamPlayers> teamPlayers_;
  /// a dependency to the position of this robot in field coordinates
  const Dependency<RobotPosition> robotPosition_;
  /// a dependency to the dimension of the field
  const Dependency<FieldDimensions> fieldDimensions_;

  /// The production of the obstacle data
  Production<TeamObstacleData> teamObstacleData_;

  /**
   * @brief integrateLocalObstacles integrate the obstacles from the local obstacle filter
   */
  void integrateLocalObstacles();
  /**
   * @brief integrateTeamPlayerKnowledge integrate the knowledge about
   * the team players from TeamPlayer data
   */
  void integrateTeamPlayerKnowledge();
  /**
   * @brief integrateTeamPlayersObstacles updates the filter with
   * the obstacles detected by a given teammate
   * @param teamPlayer the teammate who's obstacles are to be integrated
   */
  void integrateTeamPlayersObstacles(const TeamPlayer& teamPlayer);
  /**
   * @brief integrateMapObstacles integrates obstacles that are known from the world model (map)
   */
  void integrateMapObstacles();
  /**
   * @brief integrateRobotDetectionObstacles integrates obstacles from the visual robot detection
   */
  void integrateRobotDetectionObstacles();
  /**
   * @brief typeIsAtLeastAsSpecificAndMergable compares two types for mergeability
   * @param first the ObstacleType of the first obstacle
   * @param second the ObstacleType of the second obstacle
   * @return true if both types are mergable and the first one is
   * the one containing more or equal amount of information
   */
  bool typeIsAtLeastAsSpecificAndMergable(const ObstacleType first,
                                          const ObstacleType second) const;
  /**
   * @brief mapToMergedType maps a tuple of ObstacleTypes to the merge result
   * @param t1 the first ObstacleType
   * @param t2 the second ObstacleType
   * @return the merged type, INVALID if not mergeable
   */
  ObstacleType mapToMergedType(const ObstacleType t1, const ObstacleType t2) const;
  /**
   * @brief obstacleTypeIsCompatibleWithThisRobot checks wether
   * an obstacle type would be technically mergeable with this robot
   * @return true if this obstacle type could be this robot
   */
  bool obstacleTypeIsCompatibleWithThisRobot(const ObstacleType obstacleType) const;
  /**
   * @brief update integrate obstacle to teamObstacleData_
   * @param newObstaclePosition the position of the obstacle
   * @param referencePose the pose in whichs coordinates the obstaclePosition is measured
   * @param newType of given obstacle
   * @param obstacleCouldBeThisRobot defaults to true. Handle the case of
   * other robots potentially detecting this robot
   */
  void updateObstacle(const Vector2f& newObstaclePosition, const Pose& referencePose,
                      const ObstacleType newType, const bool obstacleCouldBeThisRobot = true);
};
