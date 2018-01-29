#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Pose.hpp"


class OdometryData : public DataType<OdometryData>
{
public:
  /// the pose of the robot relative to the point where it started
  Pose accumulatedOdometry;
  /**
   * @brief reset does nothing
   */
  void reset()
  {
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["accumulatedOdometry"] << accumulatedOdometry;
  }
  virtual void fromValue(const Uni::Value& value)
  {
    value["accumulatedOdometry"] >> accumulatedOdometry;
  }
};
