#pragma once

#include "Framework/DataType.hpp"

/**
 * @brief Modes that can be applied to single eyes.
 */
enum class EyeMode
{
  OFF,
  COLOR,
  RAINBOW
};

class EyeLEDRequest : public DataType<EyeLEDRequest>
{
public:
  /// the name of this DataType
  DataTypeName name = "EyeLEDRequest";
  /// The mode for the left eye LED
  EyeMode leftEyeMode;
  /// the red value of the left LED
  float leftR;
  /// the green value of the left LED
  float leftG;
  /// the blue value of the left LED
  float leftB;
  /// The mode for the right eye LED
  EyeMode rightEyeMode;
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
    leftEyeMode = EyeMode::OFF;
    leftR = 0.f;
    leftG = 0.f;
    leftB = 0.f;

    rightEyeMode = EyeMode::OFF;
    rightR = 0.f;
    rightG = 0.f;
    rightB = 0.f;
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
