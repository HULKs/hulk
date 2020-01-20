#pragma once

#include <Data/MotionOutput.hpp>

class StandUpOutput : public DataType<StandUpOutput, MotionOutput>
{
public:
  /// the name of this DataType
  DataTypeName name = "StandUpOutput";
};
