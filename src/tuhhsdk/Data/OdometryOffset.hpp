#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Pose.hpp"


/**
 * @class OdometryOffset does not work trans module manager boundaries!!!!!!!!!!!!
 */
class OdometryOffset : public DataType<OdometryOffset>
{
public:
  /// the odometry offset in this cycle
  Pose odometryOffset;
  /**
   * @brief reset resets the offset to 0
   */
  void reset()
  {
    odometryOffset = Pose();
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["odometryOffset"] << odometryOffset;
  }
  virtual void fromValue(const Uni::Value& value)
  {
    value["odometryOffset"] >> odometryOffset;
  }
};
