#pragma once

#include <vector>

#include "Framework/DataType.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Arc.hpp"
#include "Tools/Math/Line.hpp"


struct Edge : public Uni::To, public Uni::From
{
  // represents type of edge
  bool isArc;

  Arc<float> arc;
  Line<float> line;

  explicit Edge(const bool isArc = false) : isArc(isArc), arc(), line() {}

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["isArc"] << isArc;
    // CircularArc part
    value["arc"] << arc;
    // Line part
    value["line"] << line;

  }

  void fromValue(const Uni::Value& value) override
  {
    value["isArc"] >> isArc;
    // CircularArc part
    value["arc"] >> arc;
    // Line part
    value["line"] >> line;
  }
};


class Path : public DataType<Path>
{
public:
  /// the name of this DataType
  DataTypeName name = "Path";
  // Vector of edges is the path
  std::vector<Edge> edges;

  Path() = default;

  void reset() override
  {
    edges.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["edges"] << edges;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["edges"] >> edges;
  }
};
