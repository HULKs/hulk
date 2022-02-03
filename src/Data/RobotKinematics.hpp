#pragma once

#include "Framework/DataType.hpp"
#include "Hardware/Definitions.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/KinematicMatrix.hpp"
#include <array>


class RobotKinematics : public DataType<RobotKinematics>
{
public:
  /// the name of this DataType
  DataTypeName name__{"RobotKinematics"};
  /// the kinematic matrices of the joints
  JointsArray<KinematicMatrix> matrices;
  /// the kinematic matrix torso to the support foot
  KinematicMatrix torso2ground;
  /// the offset from the last ground position to this cycle's ground position
  Vector2f lastGround2currentGround;
  /// whether the torso2ground matrix is valid
  bool isTorso2groundValid;
  /// the center of mass relative to the torso
  Vector3f com = Vector3f::Zero();
  /**
   * @brief reset does nothing
   */
  void reset() override {}

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["matrices"] << matrices;
    value["torso2ground"] << torso2ground;
    value["lastGround2currentGround"] << lastGround2currentGround;
    value["torso2groundValid"] << isTorso2groundValid;
    value["com"] << com;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["matrices"] >> matrices;
    value["torso2ground"] >> torso2ground;
    value["lastGround2currentGround"] >> lastGround2currentGround;
    value["torso2groundValid"] >> isTorso2groundValid;
    value["com"] >> com;
  }
};
