#pragma once

#include "ImageSegments.hpp"

struct FilteredSegments : public DataType<FilteredSegments>
{
public:
  /// the name of this DataType
  DataTypeName name = "FilteredSegments";
  std::vector<const Segment*> vertical;
  std::vector<const Segment*> horizontal;
  bool valid = false;

  void reset() override
  {
    vertical.clear();
    horizontal.clear();
    valid = false;
  }

  void toValue(Uni::Value& /*value*/) const override {}

  void fromValue(const Uni::Value& /*value*/) override {}
};
