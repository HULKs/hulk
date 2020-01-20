#pragma once

#include "Framework/DataType.hpp"
#include "Tools/Math/Pose.hpp"


class OdometryData : public DataType<OdometryData>
{
public:
  /// the name of this DataType
  DataTypeName name = "OdometryData";
  /// the pose of the robot relative to the point where it started
  Pose accumulatedOdometry;
  /**
   * @brief reset does nothing
   */
  void reset() override
  {
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["accumulatedOdometry"] << accumulatedOdometry;
  }
  void fromValue(const Uni::Value& value) override
  {
    value["accumulatedOdometry"] >> accumulatedOdometry;
  }
};
