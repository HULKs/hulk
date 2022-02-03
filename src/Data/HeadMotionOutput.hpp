#pragma once

#include "Data/MotionOutput.hpp"
#include "Hardware/Clock.hpp"


class HeadMotionOutput : public DataType<HeadMotionOutput, MotionOutput>
{
public:
  /// the name of this DataType
  DataTypeName name__{"HeadMotionOutput"};
  /// the time when the target has been reached (only usable when atTarget is true)
  Clock::time_point timeWhenReachedTarget;
  /// true when the head is where it should be
  bool atTarget;
  /// the target, to be used in combination with atTarget
  std::array<float, 2> target = {{0.f, 0.f}};
  /**
   * @brief reset resets members
   */
  void reset() override
  {
    MotionOutput::reset();
    atTarget = false;
  }

  void toValue(Uni::Value& value) const override
  {
    MotionOutput::toValue(value);
    value["timeWhenReachedTarget"] << timeWhenReachedTarget;
    value["atTarget"] << atTarget;
    value["target"] << target;
  }

  void fromValue(const Uni::Value& value) override
  {
    MotionOutput::fromValue(value);
    value["timeWhenReachedTarget"] >> timeWhenReachedTarget;
    value["atTarget"] >> atTarget;
    value["target"] >> target;
  }
};
