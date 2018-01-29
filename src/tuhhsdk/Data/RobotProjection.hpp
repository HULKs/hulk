#pragma once

#include "Tools/Math/Line.hpp"
#include "Framework/DataType.hpp"

#include <vector>

class RobotProjection : public DataType<RobotProjection>
{
public:
  std::vector<Line<int>> lines;

  void reset()
  {
    lines.clear();
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["lines"] << lines;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["lines"] >> lines;
  }
};
