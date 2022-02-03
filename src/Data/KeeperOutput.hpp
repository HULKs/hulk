#pragma once

#include "Data/MirrorableMotionOutput.hpp"

class KeeperOutput : public DataType<KeeperOutput, MirrorableMotionOutput>
{
public:
  /// the name of this DataType
  DataTypeName name__{"KeeperOutput"};
};
