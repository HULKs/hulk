#pragma once

#include "Data/MotionOutput.hpp"

class MirrorableMotionOutput : public DataType<MirrorableMotionOutput, MotionOutput>
{
public:
  /// the name of this DataType
  DataTypeName name__{"MirrorableMotionOutput"};
  /**
   * @brief mirrorAngles mirrors the outputs body angles
   */
  void mirrorAngles();
  JointsArray<float> getMirroredAngles() const;
};
