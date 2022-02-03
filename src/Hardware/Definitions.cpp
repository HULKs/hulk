#include "Hardware/Definitions.hpp"

void FSRInfo::toValue(Uni::Value& value) const
{
  value = Uni::Value{Uni::ValueType::OBJECT};
  value["frontLeft"] << frontLeft;
  value["frontRight"] << frontRight;
  value["rearLeft"] << rearLeft;
  value["rearRight"] << rearRight;
}
void FSRInfo::fromValue(const Uni::Value& value)
{
  value["frontLeft"] >> frontLeft;
  value["frontRight"] >> frontRight;
  value["rearLeft"] >> rearLeft;
  value["rearRight"] >> rearRight;
}

void IMU::toValue(Uni::Value& value) const
{
  value = Uni::Value{Uni::ValueType::OBJECT};
  value["gyroscope"] << gyroscope;
  value["angle"] << angle;
  value["accelerometer"] << accelerometer;
}
void IMU::fromValue(const Uni::Value& value)
{
  value["gyroscope"] >> gyroscope;
  value["angle"] >> angle;
  value["accelerometer"] >> accelerometer;
}

void SonarInfo::toValue(Uni::Value& value) const
{
  value = Uni::Value{Uni::ValueType::OBJECT};
  value["leftSensor"] << leftSensor;
  value["rightSensor"] << rightSensor;
}
void SonarInfo::fromValue(const Uni::Value& value)
{
  value["leftSensor"] >> leftSensor;
  value["rightSensor"] >> rightSensor;
}

void SwitchInfo::toValue(Uni::Value& value) const
{
  value = Uni::Value{Uni::ValueType::OBJECT};
  value["isChestButtonPressed"] << isChestButtonPressed;
  value["isHeadFrontPressed"] << isHeadFrontPressed;
  value["isHeadMiddlePressed"] << isHeadMiddlePressed;
  value["isHeadRearPressed"] << isHeadRearPressed;
  value["isLeftFootLeftPressed"] << isLeftFootLeftPressed;
  value["isLeftFootRightPressed"] << isLeftFootRightPressed;
  value["isLeftHandBackPressed"] << isLeftHandBackPressed;
  value["isLeftHandLeftPressed"] << isLeftHandLeftPressed;
  value["isLeftHandRightPressed"] << isLeftHandRightPressed;
  value["isRightFootLeftPressed"] << isRightFootLeftPressed;
  value["isRightFootRightPressed"] << isRightFootRightPressed;
  value["isRightHandBackPressed"] << isRightHandBackPressed;
  value["isRightHandLeftPressed"] << isRightHandLeftPressed;
  value["isRightHandRightPressed"] << isRightHandRightPressed;
}
void SwitchInfo::fromValue(const Uni::Value& value)
{
  value["isChestButtonPressed"] >> isChestButtonPressed;
  value["isHeadFrontPressed"] >> isHeadFrontPressed;
  value["isHeadMiddlePressed"] >> isHeadMiddlePressed;
  value["isHeadRearPressed"] >> isHeadRearPressed;
  value["isLeftFootLeftPressed"] >> isLeftFootLeftPressed;
  value["isLeftFootRightPressed"] >> isLeftFootRightPressed;
  value["isLeftHandBackPressed"] >> isLeftHandBackPressed;
  value["isLeftHandLeftPressed"] >> isLeftHandLeftPressed;
  value["isLeftHandRightPressed"] >> isLeftHandRightPressed;
  value["isRightFootLeftPressed"] >> isRightFootLeftPressed;
  value["isRightFootRightPressed"] >> isRightFootRightPressed;
  value["isRightHandBackPressed"] >> isRightHandBackPressed;
  value["isRightHandLeftPressed"] >> isRightHandLeftPressed;
  value["isRightHandRightPressed"] >> isRightHandRightPressed;
}

Led::Color::Color(const float red, const float green, const float blue)
  : red{red}
  , green{green}
  , blue{blue}
{
}

Led::Color::Color(const std::uint32_t rgb)
  : red{static_cast<float>((rgb & 0xff0000u) >> 16u) / 255.f}
  , green{static_cast<float>((rgb & 0xff00u) >> 8u) / 255.f}
  , blue{static_cast<float>(rgb & 0xffu) / 255.f}
{
}

std::uint32_t Led::Color::toRGB() const
{
  return (static_cast<std::uint32_t>(red * 255.f) << 16u) |
         (static_cast<std::uint32_t>(red * 255.f) << 8u) | static_cast<std::uint32_t>(red * 255.f);
}

void Led::Color::toValue(Uni::Value& value) const
{
  value = Uni::Value{Uni::ValueType::OBJECT};
  value["red"] << red;
  value["green"] << green;
  value["blue"] << blue;
}
void Led::Color::fromValue(const Uni::Value& value)
{
  value["red"] >> red;
  value["green"] >> green;
  value["blue"] >> blue;
}

void Led::Chest::toValue(Uni::Value& value) const
{
  value = Uni::Value{Uni::ValueType::OBJECT};
  value["color"] << color;
}
void Led::Chest::fromValue(const Uni::Value& value)
{
  value["color"] >> color;
}

void Led::Ear::toValue(Uni::Value& value) const
{
  value = Uni::Value{Uni::ValueType::OBJECT};
  value["intensityAt0"] << intensityAt0;
  value["intensityAt36"] << intensityAt36;
  value["intensityAt72"] << intensityAt72;
  value["intensityAt108"] << intensityAt108;
  value["intensityAt144"] << intensityAt144;
  value["intensityAt180"] << intensityAt180;
  value["intensityAt216"] << intensityAt216;
  value["intensityAt252"] << intensityAt252;
  value["intensityAt288"] << intensityAt288;
  value["intensityAt324"] << intensityAt324;
}
void Led::Ear::fromValue(const Uni::Value& value)
{
  value["intensityAt0"] >> intensityAt0;
  value["intensityAt36"] >> intensityAt36;
  value["intensityAt72"] >> intensityAt72;
  value["intensityAt108"] >> intensityAt108;
  value["intensityAt144"] >> intensityAt144;
  value["intensityAt180"] >> intensityAt180;
  value["intensityAt216"] >> intensityAt216;
  value["intensityAt252"] >> intensityAt252;
  value["intensityAt288"] >> intensityAt288;
  value["intensityAt324"] >> intensityAt324;
}

void Led::Eye::toValue(Uni::Value& value) const
{
  value = Uni::Value{Uni::ValueType::OBJECT};
  value["colorAt0"] << colorAt0;
  value["colorAt45"] << colorAt45;
  value["colorAt90"] << colorAt90;
  value["colorAt135"] << colorAt135;
  value["colorAt180"] << colorAt180;
  value["colorAt225"] << colorAt225;
  value["colorAt270"] << colorAt270;
  value["colorAt315"] << colorAt315;
}
void Led::Eye::fromValue(const Uni::Value& value)
{
  value["colorAt0"] >> colorAt0;
  value["colorAt45"] >> colorAt45;
  value["colorAt90"] >> colorAt90;
  value["colorAt135"] >> colorAt135;
  value["colorAt180"] >> colorAt180;
  value["colorAt225"] >> colorAt225;
  value["colorAt270"] >> colorAt270;
  value["colorAt315"] >> colorAt315;
}

void Led::Foot::toValue(Uni::Value& value) const
{
  value = Uni::Value{Uni::ValueType::OBJECT};
  value["color"] << color;
}
void Led::Foot::fromValue(const Uni::Value& value)
{
  value["color"] >> color;
}
