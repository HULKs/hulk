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
  void reset()
  {
    rollPitchYaw = {};
    bodyTilt2ground = {};
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["rollPitchYaw"] << rollPitchYaw;
    value["bodyTilt2ground"] << bodyTilt2ground;
  }

  virtual void fromValue(const Uni::Value& value)
  {
    value["rollPitchYaw"] >> rollPitchYaw;
    value["bodyTilt2ground"] >> bodyTilt2ground;
  }
};
