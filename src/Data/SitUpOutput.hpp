#pragma once

#include "Data/MotionOutput.hpp"

class SitUpOutput : public DataType<SitUpOutput, MotionOutput>
{
public:
  /// the name of this DataType
  DataTypeName name__{"SitUpOutput"};
};
