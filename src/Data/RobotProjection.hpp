#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Line.hpp"

#include <vector>

class RobotProjection : public DataType<RobotProjection>
{
public:
  /// the name of this DataType
  DataTypeName name__{"RobotProjection"};
  std::vector<Line<int>> lines;

  void reset() override
  {
    lines.clear();
  }

  /*
   * @brief Checks whether a pixel is on the own robot
   * @param pos pixel position
   * @return whether the given pixel is on the robot
   */
  bool isOnRobot(const Vector2i& pos) const
  {
    for (auto& line : lines)
    {
      const int minX = std::min(line.p1.x(), line.p2.x());
      if (minX > pos.x())
      {
        continue;
      }
      const int maxX = std::max(line.p1.x(), line.p2.x());
      if (maxX < pos.x())
      {
        continue;
      }
      const int minY = std::min(line.p1.y(), line.p2.y());
      if (minY > pos.y())
      {
        continue;
      }
      const int maxY = std::max(line.p1.y(), line.p2.y());
      if (maxY < pos.y())
      {
        return true;
      }
      const float crossProduct =
          static_cast<float>(line.p2.x() - line.p1.x()) * (line.p2.y() - pos.y()) -
          (line.p2.y() - line.p1.y()) * (line.p2.x() - pos.x());
      const float sign = line.p1.x() < line.p2.x() ? 1.f : -1.f;
      if (sign * crossProduct < 0.f)
      {
        return true;
      }
    }
    return false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["lines"] << lines;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["lines"] >> lines;
  }
};
