#pragma once

#include "Data/BallState.hpp"
#include "Data/BodyPose.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/FilteredRobots.hpp"
#include "Data/FootCollisionData.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/ObstacleData.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/SonarData.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/WorldState.hpp"
#include "Framework/Module.hpp"


class Brain;

class ObstacleFilter : public Module<ObstacleFilter, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"ObstacleFilter"};
  /**
   * @brief ObstacleFilter initializes members
   * @param manager a reference to brain
   */
  explicit ObstacleFilter(const ModuleManagerInterface& manager);
  /**
   * @brief cycle updates the obstacles in the Map
   */
  void cycle() override;

private:
  // Parameters
  // The obstacle weight parameters determine how much a robot gets "pushed away" by an obstacle.
  /// flag for using sonar receiver/sensor.
  const ConditionalParameter<bool> enableSonar_;
  /// flag for using foot bumper.
  const ConditionalParameter<bool> enableFootBumper_;
  /// whether the robot detection should be enabled
  const Parameter<bool> enableRobotDetection_;
  /// The radius of a ball obstacle [m].
  const Parameter<float> ballRadius_;
  /// The radius size of the free kick area obstacle [m].
  const Parameter<float> freeKickAreaRadius_;
  /// The radius of the goal post obstacle [m].
  const Parameter<float> goalPostRadius_;
  /// The radius of a robot obstacle [m].
  const Parameter<float> robotRadius_;
  /// The radius of a fallen robot obstacle [m].
  const Parameter<float> fallenRobotRadius_;
  /// The radius of an obstacle of unknown type [m].
  const Parameter<float> unknownObstacleRadius_;
  /// All sonar obstacles detected beyond this distance are filtered out.
  const Parameter<float> ignoreSonarObstaclesBeyondDistance_;

  // Dependencies
  /// The dependency to the player configuration
  const Dependency<PlayerConfiguration> playerConfiguration_;
  /// A reference to the body pose to figure out whether we are fallen
  const Dependency<BodyPose> bodyPose_;
  /// A reference to the field dimensions for the goal free kick areas
  const Dependency<FieldDimensions> fieldDimensions_;
  /// The dependency to the game controller state
  const Dependency<GameControllerState> gameControllerState_;
  /// The dependency to the ball state, used to get the ball pose
  const Dependency<BallState> ballState_;
  /// The dependency to the team ball model
  const Dependency<TeamBallModel> teamBallModel_;
  /// The dependency to the robot data, containing relative percepts of other robots
  const Dependency<FilteredRobots> filteredRobots_;
  /// The dependency to the robot position
  const Dependency<RobotPosition> robotPosition_;
  /// The dependency to filtered sonar data
  const Dependency<SonarData> sonarData_;
  /// The dependency to the world state data
  const Dependency<WorldState> worldState_;
  /// The dependency to the foot collision data
  const Dependency<FootCollisionData> footCollisionData_;

  // Productions
  /// The production of the obstacle data
  Production<ObstacleData> obstacleData_;

  // State members
  /// used to safely update output
  bool configChanged_;

  // Functions
  /**
   * @brief Processes sonar data to create obstacles in front of the robot.
   *
   * Checks left and right sonar receiver to detect location of near obstacles.
   * The position gets calculated from the sonar values
   */
  void processSonar();

  /**
   * @brief Processes foot bumper data to create obstacles in front of the robot.
   */
  void processFootBumper();
  /**
   * @brief Processes ball position to create ball obstacles, depending on relative positioning.
   *
   * Ball is handled as an obstacle as long the robot is on the wrong side. See inline comment.
   */
  void processBall();
  /**
   * @brief Creates an obstacle around the ball when there is
   * an ongoing free kick performed by the enemy team
   */
  void processFreeKick();
  /**
   * @brief processRobots Integrates the percepts of the
   * robot detection into the local obstacle model
   */
  void processRobots();
  /**
   * @brief Updates the obstacleData on config values changes.
   */
  void updateObstacleData();
};
