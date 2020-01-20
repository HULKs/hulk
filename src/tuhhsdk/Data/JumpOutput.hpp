#pragma once

#include <Data/MirrorableMotionOutput.hpp>

class JumpOutput : public DataType<JumpOutput, MirrorableMotionOutput> {
public:
  /// the name of this DataType
  DataTypeName name = "JumpOutput";
};
