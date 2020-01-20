#pragma once

#include <vector>

#include "Framework/DataType.hpp"
#include "Tools/Math/Circle.hpp"
#include "Tools/Storage/ObjectCandidate.hpp"


class BoxCandidates : public DataType<BoxCandidates>
{
public:
  /// the name of this DataType
  DataTypeName name = "BoxCandidates";

  std::vector<ObjectCandidate> candidates;

  /// boxes for debug image
  std::vector<DebugCandidate<Circle<int>>> debugBoxes;

  /// whether the box candidates are valid
  bool valid = false;

  /**
   * @brief invalidates the position
   */
  void reset() override
  {
    valid = false;
    candidates.clear();
    debugBoxes.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["candidates"] << candidates;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["valid"] >> valid;
  }
};
