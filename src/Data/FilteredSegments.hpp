#pragma once

#include "Data/ImageSegments.hpp"

struct FilteredSegments : public DataType<FilteredSegments>
{
public:
  /// the name of this DataType
  DataTypeName name__{"FilteredSegments"};
  std::vector<const Segment*> vertical;
  std::vector<const Segment*> horizontal;
  bool valid = false;

  void reset() override
  {
    vertical.clear();
    horizontal.clear();
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& /*value*/) override {}
};
