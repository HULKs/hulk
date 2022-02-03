#pragma once

#include "Framework/DataType.hpp"

#include "Hardware/Clock.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/Pose.hpp"

class RobotPosition : public DataType<RobotPosition>
{
public:
  /// the name of this DataType
  DataTypeName name__{"RobotPosition"};
  /// the pose of the robot in SPL field coordinates
  Pose pose;
  /// the timestamp of the last time when the robot position jumped (significant changes not
  /// matching the odometry)
  Clock::time_point lastTimeJumped;
  /// whether the pose is valid
  bool valid = false;
  /**
   * @brief fieldToRobot converts field coordinates to robot coordinates
   * @param fieldCoordinates a point in field coordinates
   * @return the point in robot coordinates
   */
  Vector2f fieldToRobot(const Vector2f& fieldCoordinates) const
  {
    return pose.inverse() * fieldCoordinates;
  }
  /**
   * @brief fieldToRobot converts field coordinates to robot coordinates
   * @param fieldPose a pose in field coordinates
   * @return the pose in robot coordinates
   */
  Pose fieldToRobot(const Pose& fieldPose) const
  {
    return pose.inverse() * fieldPose;
  }
  /**
   * @brief robotToField converts robot coordinates to field coordinates
   * @param robotCoordinates a point in robot coordinates
   * @return the point in field coordinates
   */
  Vector2f robotToField(const Vector2f& robotCoordinates) const
  {
    return pose * robotCoordinates;
  }
  /**
   * @brief robotToField converts robot coordinates to field coordinates
   * @param robotPose a pose in robot coordinates
   * @return the pose in field coordinates
   */
  Pose robotToField(const Pose& robotPose) const
  {
    return pose * robotPose;
  }
  /**
   * @brief rotateFieldToRobot rotates a direction vector from field to robot coordinates (no
   * translation offset added, e.g. to convert velocities)
   * @param fieldDirection a direction vector in field coordinates
   * @return the direction vector in robot coordinates
   */
  Vector2f rotateFieldToRobot(const Vector2f& fieldDirection) const
  {
    return Rotation2Df(-pose.angle()) * fieldDirection;
  }
  /**
   * @brief rotateRobotToField rotates a direction vector from robot to field coordinates (no
   * translation offset added, e.g. to convert velocities)
   * @param robotDirection a direction vector in robot coordinates
   * @return the direction vector in field coordinates
   */
  Vector2f rotateRobotToField(const Vector2f& robotDirection) const
  {
    return Rotation2Df(pose.angle()) * robotDirection;
  }
  /**
   * @brief reset invalidates the position
   */
  void reset() override
  {
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["pose"] << pose;
    value["lastTimeJumped"] << lastTimeJumped;
    value["valid"] << valid;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["pose"] >> pose;
    value["lastTimeJumped"] >> lastTimeJumped;
    value["valid"] >> valid;
  }
};
