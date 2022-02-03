#pragma once

#include "Data/BodyDamageData.hpp"
#include "Data/ButtonData.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FSRSensorData.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/JointCalibrationData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/RobotKinematics.hpp"
#include "Data/SonarData.hpp"
#include "Framework/Module.hpp"


class Motion;

class SensorDataProvider : public Module<SensorDataProvider, Motion>
{
public:
  ModuleName name__{"SensorDataProvider"};

  explicit SensorDataProvider(const ModuleManagerInterface& manager);

  void cycle() override;

private:
  const Dependency<JointCalibrationData> jointCalibrationData_;
  const Dependency<BodyDamageData> bodyDamageData_;

  Production<FSRSensorData> fsrSensorData_;
  Production<IMUSensorData> imuSensorData_;
  Production<JointSensorData> jointSensorData_;
  Production<ButtonData> buttonData_;
  Production<SonarSensorData> sonarSensorData_;
  Production<CycleInfo> cycleInfo_;
};
