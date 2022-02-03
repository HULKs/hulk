#pragma once

#include "Data/MotionOutput.hpp"

class FallManagerOutput : public DataType<FallManagerOutput, MotionOutput>
{
public:
  /// the name of this DataType
  DataTypeName name__{"FallManagerOutput"};
  /// whether the FallManager wants to send joint commands
  bool wantToSend{false};
  /**
   * @brief reset resets members
   */
  void reset() override
  {
    MotionOutput::reset();
    wantToSend = false;
  }

  void toValue(Uni::Value& value) const override
  {
    MotionOutput::toValue(value);
    value["wantToSend"] << wantToSend;
  }

  void fromValue(const Uni::Value& value) override
  {
    MotionOutput::fromValue(value);
    value["wantToSend"] >> wantToSend;
  }
};
