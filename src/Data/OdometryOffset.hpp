#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Pose.hpp"


/**
 * @class OdometryOffset does not work trans module manager boundaries
 */
class OdometryOffset : public DataType<OdometryOffset>
{
public:
  /// the name of this DataType
  DataTypeName name__{"OdometryOffset"};
  /// the odometry offset in this cycle
  Pose odometryOffset;
  /**
   * @brief reset resets the offset to 0
   */
  void reset() override
  {
    odometryOffset = Pose();
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["odometryOffset"] << odometryOffset;
  }
  void fromValue(const Uni::Value& value) override
  {
    value["odometryOffset"] >> odometryOffset;
  }
};
