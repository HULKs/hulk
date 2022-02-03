#pragma once

#include "Data/MotionOutput.hpp"

class PuppetMotionOutput : public DataType<PuppetMotionOutput, MotionOutput>
{
public:
  /// the name of this DataType
  DataTypeName name__{"PuppetMotionOutput"};
};
