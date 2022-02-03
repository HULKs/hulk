#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/Definitions.hpp"

class PointOutput : public DataType<PointOutput>
{
public:
  /// the name of this DataType
  DataTypeName name__{"PointOutput"};
  /// whether Point wants to send joint commands
  bool wantToSend{};
  /// the left arm angles that Point wants to send
  JointsArmArray<float> leftAngles{};
  /// the right arm angles that Point wants to send
  JointsArmArray<float> rightAngles{};
  /// the stiffnesses that Point wants to send
  JointsArmArray<float> leftStiffnesses{};
  JointsArmArray<float> rightStiffnesses{};
  /**
   * @brief reset resets members
   */
  void reset() override
  {
    wantToSend = false;
    leftAngles.fill(0.f);
    rightAngles.fill(0.f);
    leftStiffnesses.fill(-1.f);
    rightStiffnesses.fill(-1.f);
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["wantToSend"] << wantToSend;
    value["leftAngles"] << leftAngles;
    value["rightAngles"] << rightAngles;
    value["leftStiffnesses"] << leftStiffnesses;
    value["rightStiffnesses"] << rightStiffnesses;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["wantToSend"] >> wantToSend;
    value["leftAngles"] >> leftAngles;
    value["rightAngles"] >> rightAngles;
    value["leftStiffnesses"] >> leftStiffnesses;
    value["rightStiffnesses"] >> rightStiffnesses;
  }
};
