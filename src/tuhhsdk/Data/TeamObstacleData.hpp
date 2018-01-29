#pragma once

#include <cmath>
#include <vector>

#include "Definitions/BHULKsStandardMessage.h"
#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"


struct TeamObstacle : public Uni::From, public Uni::To
{
  /**
   * @enum Type enumerates different types of obstacles
   */
  enum Type
  {
    /// an obstacle that is generated from the knowledge where the goal is on the map
    GOAL_POST = static_cast<int>(B_HULKs::ObstacleType::goalpost),
    /// an unknown obstacle
    UNKNOWN = static_cast<int>(B_HULKs::ObstacleType::unknown),
    /// some robot that could not be further classified
    ANONYMOUS_ROBOT = static_cast<int>(B_HULKs::ObstacleType::someRobot),
    /// a robot of the opponents team
    HOSTILE_ROBOT = static_cast<int>(B_HULKs::ObstacleType::opponent),
    /// a robot of the own team
    TEAM_ROBOT = static_cast<int>(B_HULKs::ObstacleType::teammate),
    /// same as above but fallen
    FALLEN_ANONYMOUS_ROBOT = static_cast<int>(B_HULKs::ObstacleType::fallenSomeRobot),
    /// same as above but fallen
    FALLEN_HOSTILE_ROBOT = static_cast<int>(B_HULKs::ObstacleType::fallenOpponent),
    /// same as above but fallen
    FALLEN_TEAM_ROBOT = static_cast<int>(B_HULKs::ObstacleType::fallenTeammate),
    /// the ball as obstacle for walking around the ball
    BALL,
    /// the type of invalid obstacles (for merge reasons)
    INVALID
  };
  /**
   * @brief Obstacle default constructor for serializing
   */
  TeamObstacle() = default;
  /**
   * @brief Obstacle constructor with given values for each member
   * @param relativePosition the position of the center of the obstacle in robot coordinates
   * @param absolutePosition the position of the center of the obstacle in field coordinates
   * @param type the type of the obstacle
   */
  TeamObstacle(const Vector2f& relativePosition, const Vector2f& absolutePosition, const Type type)
    : relativePosition(relativePosition)
    , absolutePosition(absolutePosition)
    , type(type)
  {
  }

  /// the position of the center of the obstacle in robot coordinates
  Vector2f relativePosition;
  /// the position of the center of the obstacle in field coordinates
  Vector2f absolutePosition;
  /// the type of the obstacle
  Type type = UNKNOWN;

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["relativePosition"] << relativePosition;
    value["absolutePosition"] << absolutePosition;
    value["type"] << type;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["relativePosition"] >> relativePosition;
    value["absolutePosition"] >> absolutePosition;
    int numberRead;
    value["type"] >> numberRead;
    type = static_cast<Type>(numberRead);
  }
};

class TeamObstacleData : public DataType<TeamObstacleData>
{
public:
  /// a vector of the team obstacles
  std::vector<TeamObstacle> teamObstacles;
  /**
   * @brief reset clears the obstacles
   */
  void reset()
  {
    teamObstacles.clear();
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["teamObstacles"] << teamObstacles;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["teamObstacles"] >> teamObstacles;
  }
};
