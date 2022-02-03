#pragma once

#include "Data/MotionOutput.hpp"
#include "Tools/Math/Angle.hpp"
#include "Tools/Math/Pose.hpp"


class WalkGeneratorOutput : public DataType<WalkGeneratorOutput, MotionOutput>
{
public:
  enum class ArmState
  {
    NORMAL,
    MOVING_BACK,
    BACK,
    MOVING_FRONT
  };

  /// the name of this DataType
  DataTypeName name__{"WalkGeneratorOutput"};
  /// the currently performed step offsets the walk manager requested to execute [m] and [rad]
  Pose requestedStepOffsets;
  /// the maximum velocity (translational and rotational) [m/s] and [rad/s]
  Pose maxVelocityComponents;
  /// whether the current step is a left phase. Left is swing foot
  bool isLeftPhase = false;
  /// the offset to the upcoming support foot
  Pose returnOffset;
  /// the current state of the arms
  ArmState armState = ArmState::NORMAL;
  /// the planed duration of the current step
  float stepDuration{0.f};
  /// the time since the last support foot change
  float t{0.f};
  /// the default duration of a single step
  float baseWalkPeriod{0.f};

  void reset() override
  {
    MotionOutput::reset();
    returnOffset = Pose{};
  }

  void toValue(Uni::Value& value) const override
  {
    MotionOutput::toValue(value);
    value["requestedStepOffsets"] << requestedStepOffsets;
    value["maxVelocityComponents"] << maxVelocityComponents;
    value["isLeftPhase"] << isLeftPhase;
    value["returnOffset"] << returnOffset;
    value["armState"] << static_cast<unsigned int>(armState);
    value["stepDuration"] << stepDuration;
    value["t"] << t;
    value["baseWalkPeriod"] << baseWalkPeriod;
  }

  void fromValue(const Uni::Value& value) override
  {
    MotionOutput::fromValue(value);
    value["requestedStepOffsets"] >> requestedStepOffsets;
    value["maxVelocityComponents"] >> maxVelocityComponents;
    value["isLeftPhase"] >> isLeftPhase;
    value["returnOffset"] >> returnOffset;
    unsigned int enumValue = 0;
    value["armState"] >> enumValue;
    armState = static_cast<ArmState>(enumValue);
    value["stepDuration"] >> stepDuration;
    value["t"] >> t;
    value["baseWalkPeriod"] >> baseWalkPeriod;
  }
};
