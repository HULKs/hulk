#pragma once

#include "Data/MotionOutput.hpp"
#include "Tools/Time.hpp"


class HeadMotionOutput : public DataType<HeadMotionOutput, MotionOutput>
{
public:
  /// the name of this DataType
  DataTypeName name = "HeadMotionOutput";
  /// the time when the target has been reached (only usable when atTarget is true)
  TimePoint timeWhenReachedTarget;
  /// true when the head is where it should be
  bool atTarget;
  /// the target, to be used in combination with atTarget
  std::array<float, 2> target = {{0.f, 0.f}};
  /**
   * @brief reset resets members
   */
  virtual void reset()
  {
    MotionOutput::reset();
    atTarget = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    MotionOutput::toValue(value);
    value["timeWhenReachedTarget"] << timeWhenReachedTarget;
    value["atTarget"] << atTarget;
    value["target"] << target;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    MotionOutput::fromValue(value);
    value["timeWhenReachedTarget"] >> timeWhenReachedTarget;
    value["atTarget"] >> atTarget;
    value["target"] >> target;
  }
};
