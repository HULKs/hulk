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
  void reset()
  {
    vertical.clear();
    horizontal.clear();
    valid = false;
  }

  /**
   * @see function in DataType
   */
  virtual void toValue(Uni::Value& /*value*/) const {}

  /**
   * @see function in DataType
   */
  virtual void fromValue(const Uni::Value& /*value*/) {}
};
