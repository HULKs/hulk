#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/Clock.hpp"
#include "Hardware/Definitions.hpp"
#include <array>


class ButtonData : public DataType<ButtonData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"ButtonData"};
  /// sensor values of the buttons (foot bumpers, chest button, head, hands)
  SwitchInfo switches;
  /// the last time when the chest button has been single pressed
  Clock::time_point lastChestButtonSinglePress;
  /// the last time when we detected a head buttons hold
  Clock::time_point lastHeadButtonsHold;
  /// whether the content is valid
  bool valid = false;
  /**
   * @brief marks the content as invalid
   */
  void reset() override
  {
    switches = SwitchInfo{};
    lastChestButtonSinglePress = Clock::time_point{};
    lastHeadButtonsHold = Clock::time_point{};
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["switches"] << switches;
    value["lastChestButtonSinglePress"] << lastChestButtonSinglePress;
    value["lastHeadButtonsHold"] << lastHeadButtonsHold;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["switches"] >> switches;
    value["lastChestButtonSinglePress"] >> lastChestButtonSinglePress;
    value["lastHeadButtonsHold"] >> lastHeadButtonsHold;
    value["valid"] >> valid;
  }
};
