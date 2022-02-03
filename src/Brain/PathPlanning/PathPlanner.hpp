#pragma once

#include "Data/ActionCommand.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/PathPlannerData.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/PlayingRoles.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/StrikerAction.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamObstacleData.hpp"
#include "Framework/Module.hpp"
#include "Libs/AStarSearch/AStarSearch.hpp"
#include "Tools/PathPlanning/PathNode.hpp"

class Brain;

/**
 * PathPlanner finds a path from the current robot position to a requested target position in
 * absolute field coordinates
 */
class PathPlanner : public Module<PathPlanner, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"PathPlanner"};

  explicit PathPlanner(const ModuleManagerInterface& manager);

  void cycle() override;

private:
  /// to get the walk target to plan the path to
  const Dependency<ActionCommand> actionCommand_;
  /// the current state of the ball filter
  const Dependency<TeamBallModel> teamBallModel_;
  /// the assignment of roles to robots
  const Dependency<PlayingRoles> playingRoles_;
  /// the position of this robot
  const Dependency<RobotPosition> robotPosition_;
  /// team obstacles obstacle to be avoided
  const Dependency<TeamObstacleData> teamObstacles_;

  Production<PathPlannerData> pathPlannerData_;

  /// an additional distance by which obstacles are moved when the start or the target is inside it
  const Parameter<float> additionalObstacleOffset_;
  /// the distance the robot starts to align with the target's orientation
  const Parameter<float> hybridAlignDistance_;
  /// if closer to the teamball than this distance, obstacles of type ROBOT are ignored in
  /// WalkModes WALK_BEHIND_BALL and DRIBBLE
  const Parameter<float> ignoreRobotObstacleDistance_;
  float ignoreRobotObstacleDistanceSquared_{0.f};
  /// decides the maximum distance to an obstacle to consider it for path planning
  const Parameter<float> maxObstacleDistance_;
  float maxObstacleDistanceSquared_{0.f};
  /// minimum length of an edge to include it in the path [m]
  const Parameter<float> minPathEdgeLength_;
  /// obstacle radius is increased by this distance to make it possible to walk around them [m]
  const Parameter<float> obstacleInflation_;

  /// the AStarSearch object which is used to find the best path
  AStarSearch<PathNode> aStarSearch_;

  /**
   * @brief Creates the obstacles in a structure for path planning
   * @param position the start position for the path to plan
   * @param target the target positoin of the path to plan
   * @return the vector of PathObstacles to be considered for path planning
   */
  std::vector<PathObstacle> createPathObstacles(const Vector2f& start, const Vector2f& target);

  /**
   * @brief interpolates between an orientation facing the walk target and the orientation to reach
   * in the end.
   * @param targetPose the Pose the robot should reach with this walk [m] and [rad]
   * @param targetAlignDistance the distance the robot should be fully aligned with the requested
   * target orientation [m]
   * @return the interpolated orientation [rad]
   */
  float hybridAlignmentAngle(const Pose& targetPose, float targetAlignDistance) const;

  /**
   * @brief Computes the next Pose to request from Motion to follow the planned path
   * @param target the target path planning is aiming for
   * @return the next pose to walk to in relative robot coordinates
   */
  Pose calculateNextPathPose(const Pose& target) const;

  /**
   * @brief Sets the given startPosition as the start of the search and the given targetPosition as
   * the target of the search
   * @param startPosition the start of the search
   * @param targetPosition the target of the search
   * @param pathObstacles the obstacles to consider for planning
   * @return whether initialization was successful
   */
  bool setStartAndTargetNode(const Vector2f& startPosition, const Vector2f& targetPosition,
                             std::vector<PathObstacle>& pathObstacles);

  /**
   * @brief Performs the actual graph seach using the A* algorithm and produces a Path object as its
   * result
   * @return a vector of PathNodes forming the optimal path
   */
  std::vector<std::shared_ptr<PathNode>> findPath();

  /**
   * @brief Takes the result of the search and places it in the pathPlannerData Production
   * @param pathNodes the search results ordered from start to target
   */
  void producePath(const std::vector<std::shared_ptr<PathNode>>& pathNodes);

  /**
   * @brief sends the created nodes for debugging
   * @param pathObstacles the obstacles used for path planning
   */
  void sendDebug(const std::vector<PathObstacle>& pathObstacles,
                 const std::vector<std::shared_ptr<PathNode>>& pathNodes) const;

  /**
   * @brief prints debug information about the created path
   */
  void printDebug();
};
