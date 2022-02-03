#pragma once

#include <cmath>
#include <vector>

#include "Data/ObstacleData.hpp"
#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"


struct TeamObstacle : public Obstacle
{
  /**
   * @brief Obstacle default constructor for serializing
   */
  TeamObstacle() = default;
  /**
   * @brief Obstacle constructor with given values for each member
   * @param relativePosition the position of the center of the obstacle in robot coordinates
   * @param absolutePosition the position of the center of the obstacle in field coordinates
   * @param radius the radius of the obstacle in meters
   * @param type the type of the obstacle
   */
  TeamObstacle(const Vector2f& relativePosition, const Vector2f& absolutePosition, float radius,
               const ObstacleType type)
    : Obstacle(relativePosition, radius, type)
    , absolutePosition(absolutePosition)
  {
  }
  /// the position of the center of the obstacle in field coordinates
  Vector2f absolutePosition;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["relativePosition"] << relativePosition;
    value["absolutePosition"] << absolutePosition;
    value["radius"] << radius;
    value["type"] << static_cast<int>(type);
  }

  void fromValue(const Uni::Value& value) override
  {
    value["relativePosition"] >> relativePosition;
    value["absolutePosition"] >> absolutePosition;
    value["radius"] >> radius;
    int numberRead;
    value["type"] >> numberRead;
    type = static_cast<ObstacleType>(numberRead);
  }
};

class TeamObstacleData : public DataType<TeamObstacleData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"TeamObstacleData"};
  /// a vector of the team obstacles
  std::vector<TeamObstacle> obstacles;
  /**
   * @brief reset clears the obstacles
   */
  void reset() override
  {
    obstacles.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["obstacles"] << obstacles;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["obstacles"] >> obstacles;
  }
};
