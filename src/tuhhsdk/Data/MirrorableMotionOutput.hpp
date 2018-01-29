#pragma once
#include <Data/MotionOutput.hpp>

class MirrorableMotionOutput : public DataType<MirrorableMotionOutput, MotionOutput>
{
public:
  /**
   * @brief mirrorAngles mirrors the outputs body angles
   */
  void mirrorAngles();
  std::vector<float> getMirroredAngles() const;
};
