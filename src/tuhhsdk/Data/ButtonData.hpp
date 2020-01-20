#pragma once

#include <array>

#include "Definitions/keys.h"
#include "Framework/DataType.hpp"
#include "Tools/Time.hpp"


class ButtonData : public DataType<ButtonData>
{
public:
  /// the name of this DataType
  DataTypeName name = "ButtonData";
  /// sensor values of the buttons (foot bumpers, chest button, head, hands)
  std::array<float, keys::sensor::SWITCH_MAX> buttons {0};
  /// the last time when the chest button has been single pressed
  TimePoint lastChestButtonSinglePress {0};
  /// the last time when the chest button has been double pressed
  TimePoint lastChestButtonDoublePress {0};
  /// whether the content is valid
  bool valid = false;
  /**
   * @brief marks the content as invalid
   */
  void reset() override
  {
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["buttons"] << buttons;
    value["lastChestButtonSinglePress"] << lastChestButtonSinglePress;
    value["lastChestButtonDoublePress"] << lastChestButtonDoublePress;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["buttons"] >> buttons;
    value["lastChestButtonSinglePress"] >> lastChestButtonSinglePress;
    value["lastChestButtonDoublePress"] >> lastChestButtonDoublePress;
    value["valid"] >> valid;
  }
};
