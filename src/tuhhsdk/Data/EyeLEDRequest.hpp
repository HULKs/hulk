#pragma once

#include "Framework/DataType.hpp"


class EyeLEDRequest : public DataType<EyeLEDRequest>
{
public:
  /// the red value of the left LED
  float leftR;
  /// the green value of the left LED
  float leftG;
  /// the blue value of the left LED
  float leftB;
  /// the red value of the right LED
  float rightR;
  /// the green value of the right LED
  float rightG;
  /// the blue value of the right LED
  float rightB;
  /**
   * @brief reset does nothing
   */
  void reset()
  {
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["leftR"] << leftR;
    value["leftG"] << leftG;
    value["leftB"] << leftB;
    value["rightR"] << rightR;
    value["rightG"] << rightG;
    value["rightB"] << rightB;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["leftR"] >> leftR;
    value["leftG"] >> leftG;
    value["leftB"] >> leftB;
    value["rightR"] >> rightR;
    value["rightG"] >> rightG;
    value["rightB"] >> rightB;
  }
};
