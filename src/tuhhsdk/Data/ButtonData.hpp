#pragma once

#include <array>

#include "Definitions/keys.h"
#include "Framework/DataType.hpp"
#include "Tools/Time.hpp"


class ButtonData : public DataType<ButtonData>
{
public:
  /// sensor values of the buttons (foot bumpers, chest button, head, hands)
  std::array<float, keys::sensor::SWITCH_MAX> buttons;
  /// the last time when the chest button has been single pressed
  TimePoint lastChestButtonSinglePress;
  /// the last time when the chest button has been double pressed
  TimePoint lastChestButtonDoublePress;
  /**
   * @brief reset does nothing
   */
  void reset()
  {
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["buttons"] << buttons;
    value["lastChestButtonSinglePress"] << lastChestButtonSinglePress;
    value["lastChestButtonDoublePress"] << lastChestButtonDoublePress;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["buttons"] >> buttons;
    value["lastChestButtonSinglePress"] >> lastChestButtonSinglePress;
    value["lastChestButtonDoublePress"] >> lastChestButtonDoublePress;
  }
};
