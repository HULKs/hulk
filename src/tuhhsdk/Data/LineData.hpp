#pragma once

#include <vector>

#include "Framework/DataType.hpp"

#include "Tools/Math/Eigen.hpp"
#include "Tools/Time.hpp"

class LineData : public DataType<LineData> {
public:
  /// the positions (robot coordinates) of the vertices of a line grid model
  VecVector2f vertices;
  /// the edges of a line grid model - each edge has two vertex indices
  VecVector2i edges;
  /// the timestamp of the image in which they were seen
  TimePoint timestamp;
  /// whether the lines are valid
  bool valid;
  /**
   * @brief reset sets the lines to a defined state
   */
  void reset()
  {
    valid = false;
    vertices.clear();
    edges.clear();
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["vertices"] << vertices;
    value["edges"] << edges;
    value["timestamp"] << timestamp;
    value["valid"] << valid;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["vertices"] >> vertices;
    value["edges"] >> edges;
    value["timestamp"] >> timestamp;
    value["valid"] >> valid;
  }
};
