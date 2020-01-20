#pragma once

#include <array>

#include "Framework/DataType.hpp"
#include "Modules/NaoProvider.h"
#include "Tools/Kinematics/KinematicMatrix.h"
#include "Tools/Math/Eigen.hpp"


class RobotKinematics : public DataType<RobotKinematics>
{
public:
  /// the name of this DataType
  DataTypeName name = "RobotKinematics";
  /// the kinematic matrices of the joints (plus torso matrix)
  std::array<KinematicMatrix, JOINTS::JOINTS_ADD_MAX> matrices;
  /// the center of mass relative to the torso
  Vector3f com = Vector3f::Zero();
  /**
   * @brief reset does nothing
   */
  void reset() override
  {
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["matrices"] << matrices;
    value["com"] << com;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["matrices"] >> matrices;
    value["com"] >> com;
  }
};
