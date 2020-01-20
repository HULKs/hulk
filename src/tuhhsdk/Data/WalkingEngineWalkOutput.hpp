#pragma once

#include "Data/MotionOutput.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Pose.hpp"


class WalkingEngineWalkOutput : public DataType<WalkingEngineWalkOutput, MotionOutput>
{
public:
  /// the name of this DataType
  DataTypeName name = "WalkingEngineWalkOutput";
  /// the offset that the walking engine thinks that it walked
  Pose stepOffset;
  /// the maximum velocities (translational and rotational)
  Pose maxVelocityComponents;
  /// the angular velocity needed to walk around the ball
  float walkAroundBallVelocity;
  /**
   * @brief reset resets the step offset to 0
   */
  void reset() override
  {
    MotionOutput::reset();
    stepOffset = Pose();
    // set maxVelocityComponents to some conservative, safe defaults
    maxVelocityComponents = Pose(0.18f, 0.1f, 36.f / TO_RAD);
  }

  void toValue(Uni::Value& value) const override
  {
    MotionOutput::toValue(value);
    value["stepOffset"] << stepOffset;
    value["maxVelocityComponents"] << maxVelocityComponents;
  }

  void fromValue(const Uni::Value& value) override
  {
    MotionOutput::fromValue(value);
    value["stepOffset"] >> stepOffset;
    value["maxVelocityComponents"] >> maxVelocityComponents;
  }
};
