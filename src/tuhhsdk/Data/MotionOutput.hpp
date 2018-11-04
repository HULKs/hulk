#pragma once
#include "Framework/DataType.hpp"
#include <vector>

/**
 * Motion outputs can inherit the mirrorAngles function in order to compute mirrored angles
 */
class MotionOutput : public DataType<MotionOutput> {
public:
  /// the name of this DataType
  DataTypeName name = "MotionOutput";
  /// whether it is safe to exit the motion
  bool safeExit;
  /// the angles that the output wants to send
  std::vector<float> angles;
  /// the stiffnesses that the output wants to send
  std::vector<float> stiffnesses;
  /**
   * @brief reset resets members
   */
  virtual void reset() {
    safeExit = false;
    angles.clear();
    stiffnesses.clear();
  }

  virtual void toValue(Uni::Value& value) const {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["safeExit"] << safeExit;
    value["angles"] << angles;
    value["stiffnesses"] << stiffnesses;
  }

  virtual void fromValue(const Uni::Value& value) {
    value["safeExit"] >> safeExit;
    value["angles"] >> angles;
    value["stiffnesses"] >> stiffnesses;
  }
};
