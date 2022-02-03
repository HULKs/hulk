#pragma once
#include "Framework/DataType.hpp"
#include "Hardware/Definitions.hpp"
#include <vector>

/**
 * Motion outputs can inherit the mirrorAngles function in order to compute mirrored angles
 */
class MotionOutput : public DataType<MotionOutput>
{
public:
  /// the name of this DataType
  DataTypeName name__{"MotionOutput"};
  /// whether it is safe to exit the motion
  bool safeExit{false};
  /// the angles that the output wants to send
  JointsArray<float> angles{};
  /// the stiffnesses that the output wants to send
  JointsArray<float> stiffnesses{};
  /// wether this data type holds valid data
  bool valid{false};
  /**
   * @brief reset resets members
   */
  void reset() override
  {
    safeExit = false;
    angles = JointsArray<float>{};
    stiffnesses = JointsArray<float>{};
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["safeExit"] << safeExit;
    value["angles"] << angles;
    value["stiffnesses"] << stiffnesses;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["safeExit"] >> safeExit;
    value["angles"] >> angles;
    value["stiffnesses"] >> stiffnesses;
    value["valid"] >> valid;
  }
};
