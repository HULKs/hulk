#pragma once

#include <Data/MotionOutput.hpp>

class PoserOutput : public DataType<PoserOutput, MotionOutput>
{
public:
  /// the name of this DataType
  DataTypeName name = "PoserOutput";
};
