#include "JointDiffProvider.hpp"

#include "Modules/NaoProvider.h"

JointDiffProvider::JointDiffProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , jointSensorData_(*this)
  , motionState_(*this)
  , jointDiff_(*this)
{
}

void JointDiffProvider::cycle()
{
  if (!jointSensorData_->valid)
  {
    return;
  }

  std::array<float, JOINTS::JOINTS_MAX> jointDiff;
  for (unsigned int i = 0; i < JOINTS::JOINTS_MAX; i++)
  {
    jointDiff[i] = std::abs(motionState_->angles[i] - jointSensorData_->angles[i]);
  }
  jointDiff_->angles = jointDiff;

  for (unsigned int i = 0; i < JOINTS_L_ARM::L_ARM_MAX; i++)
  {
    jointDiff_->leftArmSum += jointDiff[JOINTS::L_SHOULDER_PITCH + i];
    jointDiff_->leftLegSum += jointDiff[JOINTS::L_HIP_YAW_PITCH + i];
    jointDiff_->rightArmSum += jointDiff[JOINTS::R_SHOULDER_PITCH + i];
    jointDiff_->rightLegSum += jointDiff[JOINTS::R_HIP_YAW_PITCH + i];
  }
  jointDiff_->bodySum = jointDiff_->leftArmSum + jointDiff_->leftLegSum + jointDiff_->rightArmSum +
                        jointDiff_->rightLegSum;
  jointDiff_->headSum = jointDiff[JOINTS::HEAD_PITCH] + jointDiff[JOINTS::HEAD_YAW];
  jointDiff_->valid = true;
}
