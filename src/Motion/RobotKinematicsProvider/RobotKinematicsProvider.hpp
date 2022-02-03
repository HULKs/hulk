#pragma once

#include "Data/BodyPose.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/RobotKinematics.hpp"
#include "Framework/Module.hpp"

class Motion;

class RobotKinematicsProvider : public Module<RobotKinematicsProvider, Motion>
{
public:
  /// the name of this module
  ModuleName name__{"RobotKinematicsProvider"};
  explicit RobotKinematicsProvider(const ModuleManagerInterface& manager);
  /**
   * @brief cycle calculates all robot kinematics based on current sensor readings
   */
  void cycle() override;

private:
  const Dependency<BodyPose> bodyPose_;
  const Dependency<IMUSensorData> imuSensorData_;
  const Dependency<JointSensorData> jointSensorData_;
  Production<RobotKinematics> robotKinematics_;

  Vector3f lastLeft2RightFootXY_{Vector3f::Zero()};
};
