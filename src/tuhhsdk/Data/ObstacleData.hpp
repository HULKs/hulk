#pragma once

#include <cmath>
#include <vector>

#include "Definitions/BHULKsStandardMessage.h"
#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"

/**
 * @enum ObstacleType enumerates different types of obstacles
 * @brief This is enum is used for all obstacles models (local and team).
 *        Note that you may have to add obstacle types in the (B-)HULKs-Message
 *        Attention: don't f***ING change the order
 *        Attention again: When adding types here you have to check both the ObstacleFilter and the
 *        TeamObstacleFilter
 */
enum class ObstacleType
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
  /// the area to keep clear during a free kick performed by the enemy team
  FREE_KICK_AREA,
  /// the type of invalid obstacles (for merge reasons)
  INVALID,
  /// denotes the last entry of the enum class, so leave this as the last entry!
  OBSTACLETYPE_MAX
};

struct Obstacle : public Uni::From, public Uni::To
{
  /**
   * @brief Obstacle default constructor for serializing
   */
  Obstacle() = default;
  /**
   * @brief Obstacle constructor with given values for each member
   * @param relPosition the position of the center of the obstacle relative to the robot
   * @param type the type of the obstacle
   * @param radius the radius of the obstacle
   */
  Obstacle(const Vector2f& relPosition, const float radius, const ObstacleType type)
    : type(type)
    , relativePosition(relPosition)
    , radius(radius)
  {
  }

  /// the type of the obstacle
  ObstacleType type = ObstacleType::UNKNOWN;
  /// the position of the center of the obstacle relative to the robot
  Vector2f relativePosition;
  /// the radius of the obstacle
  float radius;

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["relativePosition"] << relativePosition;
    value["type"] << static_cast<int>(type);
    value["radius"] << radius;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["relativePosition"] >> relativePosition;
    int numberRead;
    value["type"] >> numberRead;
    type = static_cast<ObstacleType>(numberRead);
    value["radius"] >> radius;
  }
};

class ObstacleData : public DataType<ObstacleData>
{
public:
  /// the name of this DataType
  DataTypeName name = "ObstacleData";
  /// the list of obstacles
  std::vector<Obstacle> obstacles;
  /**
   * @brief Holds the preconfigured radius for each obstacle type.
   * Use the function below for a better readable access.
   */
  std::array<float, static_cast<int>(ObstacleType::OBSTACLETYPE_MAX) > typeRadius;
  /**
   * @brief Query the preconfigured radius of an obstacle type.
   * This provides a more readable alternative to the direct array access.
   * @param type ObstacleType for which the radius is return
   * @return preconfigured radius of given obstacle type
   */
  float typeToRadius(const ObstacleType type) const
  {
    return typeRadius.at(static_cast<int>(type));
  }
  /**
   * @brief reset clears the obstacles
   */
  void reset()
  {
    obstacles.clear();
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["obstacles"] << obstacles;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["obstacles"] >> obstacles;
  }
};
