#pragma once

#include <Data/MotionOutput.hpp>

class FallManagerOutput : public DataType<FallManagerOutput, MotionOutput> {
public:
  /// the name of this DataType
  DataTypeName name = "FallManagerOutput";
  /// whether the FallManager wants to send joint commands
  bool wantToSend;
  /**
   * @brief reset resets members
   */
  void reset()
  {
    MotionOutput::reset();
    wantToSend = false;
  }

  virtual void toValue(Uni::Value& value) const
  {
    MotionOutput::toValue(value);
    value["wantToSend"] << wantToSend;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    MotionOutput::fromValue(value);
    value["wantToSend"] >> wantToSend;
  }
};
