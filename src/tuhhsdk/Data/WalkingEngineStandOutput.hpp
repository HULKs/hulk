#pragma once

#include <Data/MotionOutput.hpp>

class WalkingEngineStandOutput : public DataType<WalkingEngineStandOutput, MotionOutput> {
public:
  virtual void reset()
  {
    MotionOutput::reset();
    // Standing is always safe to exit
    safeExit = true;
  }
};
