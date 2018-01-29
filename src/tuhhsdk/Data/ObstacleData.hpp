#pragma once

#include <cmath>
#include <vector>

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"


struct Obstacle : public Uni::From, public Uni::To
{
  /**
   * @enum Type enumerates different types of obstacles
   */
  enum Type
  {
    /// an unknown obstacle
    UNKNOWN,
    /// a sonar obstacle
    SONAR,
    /// the ball as obstacle for walking around the ball
    BALL
  };
  /**
   * @brief Obstacle default constructor for serializing
   */
  Obstacle() = default;
  /**
   * @brief Obstacle constructor with given values for each member
   * @param position the position of the center of the obstacle relative to the robot
   * @param type the type of the obstacle
   * @param radius the radius of the obstacle
   */
  Obstacle(const Type type, const Vector2f& position, const float radius)
    : type(type)
    , position(position)
    , radius(radius)
  {
  }

  /// the type of the obstacle
  Type type = UNKNOWN;
  /// the position of the center of the obstacle relative to the robot
  Vector2f position;
  /// the radius of the obstacle
  float radius = 0.f;

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["position"] << position;
    value["type"] << type;
    value["radius"] << radius;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["position"] >> position;
    int numberRead;
    value["type"] >> numberRead;
    type = static_cast<Type>(numberRead);
    value["radius"] >> radius;
  }
};

class ObstacleData : public DataType<ObstacleData>
{
public:
  /// the list of obstacles
  std::vector<Obstacle> obstacles;
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
