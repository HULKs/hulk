#pragma once

#include <set>

#include "Framework/DataType.hpp"
#include "Tools/Math/Circle.hpp"

class PerspectiveGridCandidates : public DataType<PerspectiveGridCandidates>
{
public:
  /// the name of this DataType
  DataTypeName name__{"PerspectiveGridCandidates"};

  /// whether this datatype contains valid data
  bool valid{false};

  /// comparator for Circle<int>, used for sorting in std::set for the candidates member
  struct CircleCompare
  {
    bool operator()(const Circle<int>& lhs, const Circle<int>& rhs) const
    {
      return lhs.center.y() < rhs.center.y() ||
             (lhs.center.y() == rhs.center.y() && lhs.center.x() < rhs.center.x());
    }
  };

  /// x: 422, y: 444, radius: 444
  std::set<Circle<int>, CircleCompare> candidates;

  /**
   * @brief invalidates the position
   */
  void reset() override
  {
    valid = false;
    candidates.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["valid"] << valid;
    value["candidates"] << candidates;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["valid"] >> valid;
    value["candidates"] >> candidates;
  }
};
