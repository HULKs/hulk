#pragma once
#include "Framework/DataType.hpp"
#include <vector>

/**
 * Motion outputs can inherit the mirrorAngles function in order to compute mirrored angles
 */
class MotionOutput : public DataType<MotionOutput>
{
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
  void reset() override
  {
    safeExit = false;
    angles.clear();
    stiffnesses.clear();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["safeExit"] << safeExit;
    value["angles"] << angles;
    value["stiffnesses"] << stiffnesses;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["safeExit"] >> safeExit;
    value["angles"] >> angles;
    value["stiffnesses"] >> stiffnesses;
  }
};
