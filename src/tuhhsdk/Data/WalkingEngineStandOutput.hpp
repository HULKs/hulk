#pragma once

#include <Data/MotionOutput.hpp>

class WalkingEngineStandOutput : public DataType<WalkingEngineStandOutput, MotionOutput> {
public:
  /// the name of this DataType
  DataTypeName name = "WalkingEngineStandOutput";
  virtual void reset()
  {
    MotionOutput::reset();
    // Standing is always safe to exit
    safeExit = true;
  }
};
