#pragma once

#include "Data/FieldDimensions.hpp"
#include "Framework/DataType.hpp"
#include "Tools/Math/Arc.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Geometry.hpp"
#include "Tools/Math/Line.hpp"
#include "Tools/PathPlanning/PathNode.hpp"
#include <vector>

/**
 * @brief Structure to hold a path consisting of Edges (either straight lines or arcs)
 */
struct Path : public Uni::To
{
public:
  /// Vector of edges is the path
  std::vector<std::variant<Line<float>, Arc<float>>> edges;
  /// Total cost of path
  float cost{std::numeric_limits<float>::max()};
  /// The starting position of the path
  Vector2f absoluteStartPosition;
  /// The target of the path
  Vector2f absoluteTargetPosition;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    auto edgesValue = Uni::Value(Uni::ValueType::ARRAY);
    edgesValue.reserve(edges.size());
    Uni::Value::valuesVector_t::size_type i = 0;
    for (const auto& edge : edges)
    {
      std::visit([&](auto& arg) { edgesValue[i++] << arg; }, edge);
    }
    value["edges"] << edgesValue;
    value["cost"] << cost;
    value["absoluteStartPosition"] << absoluteStartPosition;
    value["absoluteTargetPosition"] << absoluteTargetPosition;
  }

  void reset()
  {
    edges.clear();
    cost = std::numeric_limits<float>::max();
    absoluteStartPosition = Vector2f::Zero();
    absoluteTargetPosition = Vector2f::Zero();
  }
};


/**
 * @brief Data produced by the PathPlanner in Brain
 */
class PathPlannerData : public DataType<PathPlannerData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"PathPlannerData"};

  /// the path representing the optimal path from start to target
  Path path;
  /// the next pose relative to the robot on the path to be reached
  Pose nextRelativePathPose;

  void reset() override
  {
    path.reset();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["path"] << path;
    value["nextRelativePathPose"] << nextRelativePathPose;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["nextRelativePathPose"] >> nextRelativePathPose;
  }
};
