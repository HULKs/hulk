#pragma once

#include <vector>

#include "Framework/DataType.hpp"

class PointOutput : public DataType<PointOutput> {
public:
  /// the name of this DataType
  DataTypeName name = "PointOutput";
  /// whether Point wants to send joint commands
  bool wantToSend;
  /// the left arm angles that Point wants to send
  std::vector<float> leftAngles;
  /// the right arm angles that Point wants to send
  std::vector<float> rightAngles;
  /// the stiffnesses that Point wants to send
  std::vector<float> stiffnesses;
  /**
   * @brief reset resets members
   */
  void reset() override
  {
    wantToSend = false;
    leftAngles.clear();
    rightAngles.clear();
    stiffnesses.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["wantToSend"] << wantToSend;
    value["leftAngles"] << leftAngles;
    value["rightAngles"] << rightAngles;
    value["stiffnesses"] << stiffnesses;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["wantToSend"] >> wantToSend;
    value["leftAngles"] >> leftAngles;
    value["rightAngles"] >> rightAngles;
    value["stiffnesses"] >> stiffnesses;
  }
};
