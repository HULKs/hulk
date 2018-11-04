#pragma once

#include <limits>
#include <vector>

#include "Tools/Math/Eigen.hpp"

#include "Data/BallState.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/PlayerConfiguration.hpp"
#include "Data/RobotPosition.hpp"
#include "Data/TeamBallModel.hpp"
#include "Data/TeamPlayers.hpp"
#include "Framework/Module.hpp"


class Brain;

class TeamBallFilter : public Module<TeamBallFilter, Brain>
{
public:
  /// the name of this module
  ModuleName name = "TeamBallFilter";
  /**
   * @brief TeamBallFilter creates a model of the ball as seen by the complete team
   * @param manager a reference to brain
   */
  TeamBallFilter(const ModuleManagerInterface& manager);
  /**
   * @brief cycle
   */
  void cycle();

private:
  struct Ball
  {
    /// the position of the ball
    Vector2f position;
    /// the velocity of the ball
    Vector2f velocity;
  };
  struct TeamPlayerBall
  {
    /// the time when the ball has been seen
    TimePoint timestamp;
    /// the number of the player
    unsigned int playerNumber;
    /// shortest distance from where the ball was seen
    float distance;
    /// the absolute position where the teammate saw the ball
    Ball ball;
  };
  struct BallCluster
  {
    /// the balls that belong to this cluster (pointers to contents of ballBuffer_)
    std::vector<TeamPlayerBall*> balls;
    /// whether the cluster contains the own ball
    bool containsOwnBall = false;
    /// the closest distance between a robot and the ball in this cluster
    float closestBallDistance = std::numeric_limits<float>::max();
  };
  /**
   * @brief addBallToBuffer updates an already existing ball or adds a new ball to the ballBuffer_
   * @param playerNumber number of the player who wants to add the ball to the buffer
   * @param pose the pose of the player who wants to add the ball to the buffer
   * @param relBallPosition relative position to the seen ball
   * @param relBallVelocity relative ball velocity of the seen ball
   * @param timestamp the time when the ball was seen
   */
  void addBallToBuffer(const unsigned int playerNumber, const Pose& pose, const Vector2f& relBallPosition, const Vector2f& relBallVelocity,
                       const TimePoint timestamp);
  /**
   * @brief updateBallBuffer adds and removes balls to/from the buffer
   */
  void updateBallBuffer();
  /// the age that a ball can have at maximum to be added to the buffer
  const Parameter<float> maxAddAge_;
  /// the minimum wait after accepting a ball of a recently jumped robot
  const Parameter<float> minWaitAfterJumpToAddBall_;
  /// the velocity that a ball may have to be added to the buffer
  const Parameter<float> maxBallVelocity_;
  /// the age that a ball must have to be removed from the buffer
  const Parameter<float> minRemoveAge_;
  /// the maximum distance for two balls (in field coordinates) to be considered the same
  const Parameter<float> maxCompatibilityDistance_;
  /// parameter for tolerance in isInsideField
  const Parameter<float> insideFieldTolerance_;

  const Dependency<PlayerConfiguration> playerConfiguration_;
  const Dependency<RobotPosition> robotPosition_;
  const Dependency<BallState> ballState_;
  const Dependency<TeamPlayers> teamPlayers_;
  const Dependency<FieldDimensions> fieldDimensions_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<GameControllerState> gameControllerState_;

  /// the generated team ball model
  Production<TeamBallModel> teamBallModel_;
  /// an internal buffer of balls that team members have seen
  std::vector<TeamPlayerBall> ballBuffer_;
};
