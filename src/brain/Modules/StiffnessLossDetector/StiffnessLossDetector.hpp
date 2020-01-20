#pragma once

#include "Data/JointDiff.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/MotionState.hpp"
#include "Data/StiffnessLoss.hpp"
#include "Framework/Module.hpp"

class Brain;

/**
 * @brief detect stiffness loss in joints
 *
 * The StiffnessLossDetector detects joints that do not respond to joint commands (stiffness loss).
 * Stiffness loss is detected if the joint diff exceeds a threshold and the current deceeds a
 * threshold. Each joint is checked individually. Joints can be disabled.
 */
class StiffnessLossDetector : public Module<StiffnessLossDetector, Brain>
{
public:
  ModuleName name = "StiffnessLossDetector";
  explicit StiffnessLossDetector(const ModuleManagerInterface& manager);
  void cycle() override;

private:
  const Dependency<JointDiff> jointDiff_;
  const Dependency<JointSensorData> jointSensorData_;
  const Dependency<MotionState> motionState_;

  Production<StiffnessLoss> stiffnessLoss_;

  // which joints NOT to check for stiffness los
  const Parameter<std::vector<bool>> disabledJoints_;
  // angle threshold to detect stiffness loss
  Parameter<float> stiffnessLossAngleThreshold_;
  // current threshold to detect stiffness loss
  const Parameter<float> stiffnessLossCurrentThreshold_;
  // maximum number of misses before resetting the hits count
  const Parameter<unsigned int> maxNumMisses_;
  // minimum number of hits before producing stiffness loss
  const Parameter<unsigned int> numHitsForDetection_;

  // number of cycles in which a loss was observed
  std::array<unsigned int, JOINTS::JOINTS_MAX> hits_;
  // number of cycles since  a loss was observed
  std::array<unsigned int, JOINTS::JOINTS_MAX> misses_;
};
