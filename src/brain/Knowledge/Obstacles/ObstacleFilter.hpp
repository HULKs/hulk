#pragma once

#include "Data/BallState.hpp"
#include "Data/MotionRequest.hpp"
#include "Data/ObstacleData.hpp"
#include "Data/SonarData.hpp"
#include "Framework/Module.hpp"


class Brain;

class ObstacleFilter : public Module<ObstacleFilter, Brain>
{
public:
  /**
   * @brief ObstacleFilter initializes members
   * @param manager a reference to brain
   */
  ObstacleFilter(const ModuleManagerInterface& manager);
  /**
   * @brief cycle updates the obstacles in the Map
   */
  void cycle();

private:
  // Parameters

  /// flag for using sonar receiver/sensor
  Parameter<bool> enableSonar_;
  /// The physical size of a ball obstacle [m]. This is a different value than the avoidance radius defined in MotionPlanner!
  Parameter<float> ballObstacleRadius_;
  /// The physical size of a sonar obstacle [m]. This is a different value than the avoidance radius defined in MotionPlanner!
  Parameter<float> sonarObstacleRadius_;

  /**
   * Distance beyond all obstacle get ignored.
   */
  const Parameter<float> ignoreSonarObstaclesBeyondDistance_;
  // Dependencies

  /// The dependency to the motion request
  const Dependency<MotionRequest> motionRequest_;
  /// The dependency to the ball state, used to get the ball pose
  const Dependency<BallState> ballState_;
  /// The dependency to filtered sonar data
  const Dependency<SonarData> sonarData_;

  /// The production of the obstacle data
  Production<ObstacleData> obstacleData_;

  /**
   * @brief Processes sonar data to create obstacles in front of the robot.
   *
   * Checks left and right sonar receiver to detect location of near obstacles.
   * The position gets calculated from the sonar values
   */
  void processSonar();

  /**
   * @brief Processes ball position to create ball obstacles, depending on relative positioning.
   *
   * Ball is handled as an obstacle as long the robot is on the wrong side. See inline comment.
   */
  void processBall();

  /**
   * @brief Checks if distance is in the reliable range(config parameter, ignoreObstaclesBeyondDistance)
   * @param distance sensor reading
   * @return isObstacle
   */
  bool sonarGetIsObstacle(const float distance) const;
};
