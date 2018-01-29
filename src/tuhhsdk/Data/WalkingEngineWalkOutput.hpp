#pragma once

#include "Data/MotionOutput.hpp"
#include "Tools/Math/Pose.hpp"


class WalkingEngineWalkOutput : public DataType<WalkingEngineWalkOutput, MotionOutput> {
public:
  /// the offset that the walking engine thinks that it walked
  Pose stepOffset;
  /**
   * @brief reset resets the step offset to 0
   */
  void reset()
  {
    MotionOutput::reset();
    stepOffset = Pose();
  }

  virtual void toValue(Uni::Value& value) const
  {
    MotionOutput::toValue(value);
    value["stepOffset"] << stepOffset;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    MotionOutput::fromValue(value);
    value["stepOffset"] >> stepOffset;
  }
};
