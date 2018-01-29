#pragma once

#include <vector>

#include "Framework/DataType.hpp"

class PointOutput : public DataType<PointOutput> {
public:
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
  void reset()
  {
    wantToSend = false;
    leftAngles.clear();
    rightAngles.clear();
    stiffnesses.clear();
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["wantToSend"] << wantToSend;
    value["leftAngles"] << leftAngles;
    value["rightAngles"] << rightAngles;
    value["stiffnesses"] << stiffnesses;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["wantToSend"] >> wantToSend;
    value["leftAngles"] >> leftAngles;
    value["rightAngles"] >> rightAngles;
    value["stiffnesses"] >> stiffnesses;
  }
};
