#include "Motion/JointDiffProvider/JointDiffProvider.hpp"

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

  for (std::size_t i = 0; i < static_cast<std::size_t>(Joints::MAX); i++)
  {
    jointDiff_->angles[static_cast<Joints>(i)] =
        std::abs(motionState_->angles[static_cast<Joints>(i)] -
                 jointSensorData_->angles[static_cast<Joints>(i)]);
  }

  jointDiff_->headSum =
      jointDiff_->angles[Joints::HEAD_YAW] + jointDiff_->angles[Joints::HEAD_PITCH];
  jointDiff_->leftLegSum =
      jointDiff_->angles[Joints::L_HIP_YAW_PITCH] + jointDiff_->angles[Joints::L_HIP_ROLL] +
      jointDiff_->angles[Joints::L_HIP_PITCH] + jointDiff_->angles[Joints::L_KNEE_PITCH] +
      jointDiff_->angles[Joints::L_ANKLE_PITCH] + jointDiff_->angles[Joints::L_ANKLE_ROLL];
  jointDiff_->rightLegSum =
      jointDiff_->angles[Joints::R_HIP_YAW_PITCH] + jointDiff_->angles[Joints::R_HIP_ROLL] +
      jointDiff_->angles[Joints::R_HIP_PITCH] + jointDiff_->angles[Joints::R_KNEE_PITCH] +
      jointDiff_->angles[Joints::R_ANKLE_PITCH] + jointDiff_->angles[Joints::R_ANKLE_ROLL];
  jointDiff_->leftArmSum =
      jointDiff_->angles[Joints::L_SHOULDER_PITCH] + jointDiff_->angles[Joints::L_SHOULDER_ROLL] +
      jointDiff_->angles[Joints::L_ELBOW_YAW] + jointDiff_->angles[Joints::L_ELBOW_ROLL] +
      jointDiff_->angles[Joints::L_WRIST_YAW] + jointDiff_->angles[Joints::L_HAND];
  jointDiff_->rightArmSum =
      jointDiff_->angles[Joints::R_SHOULDER_PITCH] + jointDiff_->angles[Joints::R_SHOULDER_ROLL] +
      jointDiff_->angles[Joints::R_ELBOW_YAW] + jointDiff_->angles[Joints::R_ELBOW_ROLL] +
      jointDiff_->angles[Joints::R_WRIST_YAW] + jointDiff_->angles[Joints::R_HAND];
  jointDiff_->bodySum = jointDiff_->leftArmSum + jointDiff_->leftLegSum + jointDiff_->rightArmSum +
                        jointDiff_->rightLegSum;
  jointDiff_->valid = true;
}
