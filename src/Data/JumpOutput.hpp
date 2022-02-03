#pragma once

#include "Data/MirrorableMotionOutput.hpp"

class JumpOutput : public DataType<JumpOutput, MirrorableMotionOutput>
{
public:
  /// the name of this DataType
  DataTypeName name__{"JumpOutput"};
  enum class Type
  {
    NONE,
    SQUAT,
    TAKE_LEFT,
    TAKE_RIGHT,
    JUMP_LEFT,
    JUMP_RIGHT,
    MAX
  };
};
