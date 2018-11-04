#pragma once

#include <Data/MirrorableMotionOutput.hpp>

class KickOutput : public DataType<KickOutput, MirrorableMotionOutput> {
public:
  /// the name of this DataType
  DataTypeName name = "KickOutput";
};
