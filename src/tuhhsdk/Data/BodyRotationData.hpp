#pragma once

#include <Framework/DataType.hpp>

#include "Tools/Math/Eigen.hpp"

class BodyRotationData : public DataType<BodyRotationData> {
public:
  /// the name of this DataType
  DataTypeName name = "BodyRotationData";
  /// the rotation of the body in terms of roll, pitch and yaw angles (equivalent to IMUSensorData::angle)
  Vector3f rollPitchYaw;
  /// a rotation matrix rotating the body coordinate system into ground coordinates (containing pitch and roll)
  AngleAxisf bodyTilt2ground;
  /**
   * @brief reset sets the state to some defaults
   */
  void reset() override
  {
    rollPitchYaw = {};
    bodyTilt2ground = {};
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["rollPitchYaw"] << rollPitchYaw;
    value["bodyTilt2ground"] << bodyTilt2ground;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["rollPitchYaw"] >> rollPitchYaw;
    value["bodyTilt2ground"] >> bodyTilt2ground;
  }
};
