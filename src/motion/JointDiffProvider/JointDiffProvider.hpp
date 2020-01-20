#pragma once

#include "Data/JointDiff.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/MotionState.hpp"
#include "Framework/Module.hpp"

class Motion;

class JointDiffProvider : public Module<JointDiffProvider, Motion>
{
public:
  ModuleName name = "JointDiffProvider";
  explicit JointDiffProvider(const ModuleManagerInterface& module);
  void cycle() override;

private:
  const Dependency<JointSensorData> jointSensorData_;
  const Dependency<MotionState> motionState_;

  Production<JointDiff> jointDiff_;
};
