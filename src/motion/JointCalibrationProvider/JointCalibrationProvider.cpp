#include "JointCalibrationProvider.hpp"
#include "Tools/Math/Angle.hpp"

JointCalibrationProvider::JointCalibrationProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , headYaw_(*this, "headYaw", [this]() { configChanged_ = true; })
  , headPitch_(*this, "headPitch", [this]() { configChanged_ = true; })
  , lShoulderPitch_(*this, "lShoulderPitch", [this]() { configChanged_ = true; })
  , lShoulderRoll_(*this, "lShoulderRoll", [this]() { configChanged_ = true; })
  , lElbowYaw_(*this, "lElbowYaw", [this]() { configChanged_ = true; })
  , lElbowRoll_(*this, "lElbowRoll", [this]() { configChanged_ = true; })
  , lWristYaw_(*this, "lWristYaw", [this]() { configChanged_ = true; })
  , lHand_(*this, "lHand", [this]() { configChanged_ = true; })
  , lHipYawPitch_(*this, "lHipYawPitch", [this]() { configChanged_ = true; })
  , lHipRoll_(*this, "lHipRoll", [this]() { configChanged_ = true; })
  , lHipPitch_(*this, "lHipPitch", [this]() { configChanged_ = true; })
  , lKneePitch_(*this, "lKneePitch", [this]() { configChanged_ = true; })
  , lAnklePitch_(*this, "lAnklePitch", [this]() { configChanged_ = true; })
  , lAnkleRoll_(*this, "lAnkleRoll", [this]() { configChanged_ = true; })
  , rHipYawPitch_(*this, "rHipYawPitch", [this]() { configChanged_ = true; })
  , rHipRoll_(*this, "rHipRoll", [this]() { configChanged_ = true; })
  , rHipPitch_(*this, "rHipPitch", [this]() { configChanged_ = true; })
  , rKneePitch_(*this, "rKneePitch", [this]() { configChanged_ = true; })
  , rAnklePitch_(*this, "rAnklePitch", [this]() { configChanged_ = true; })
  , rAnkleRoll_(*this, "rAnkleRoll", [this]() { configChanged_ = true; })
  , rShoulderPitch_(*this, "rShoulderPitch", [this]() { configChanged_ = true; })
  , rShoulderRoll_(*this, "rShoulderRoll", [this]() { configChanged_ = true; })
  , rElbowYaw_(*this, "rElbowYaw", [this]() { configChanged_ = true; })
  , rElbowRoll_(*this, "rElbowRoll", [this]() { configChanged_ = true; })
  , rWristYaw_(*this, "rWristYaw", [this]() { configChanged_ = true; })
  , rHand_(*this, "rHand", [this]() { configChanged_ = true; })
  , configChanged_(true)
  , jointCalibrationData_(*this)
{
}

void JointCalibrationProvider::cycle()
{
  if (configChanged_)
  {
    updateOutput();
    configChanged_ = false;
  }
}

void JointCalibrationProvider::updateOutput()
{
  jointCalibrationData_->calibrationOffsets[JOINTS::HEAD_YAW] = headYaw_();
  jointCalibrationData_->calibrationOffsets[JOINTS::HEAD_PITCH] = headPitch_();
  jointCalibrationData_->calibrationOffsets[JOINTS::L_SHOULDER_PITCH] = lShoulderPitch_();
  jointCalibrationData_->calibrationOffsets[JOINTS::L_SHOULDER_ROLL] = lShoulderRoll_();
  jointCalibrationData_->calibrationOffsets[JOINTS::L_ELBOW_YAW] = lElbowYaw_();
  jointCalibrationData_->calibrationOffsets[JOINTS::L_ELBOW_ROLL] = lElbowRoll_();
  jointCalibrationData_->calibrationOffsets[JOINTS::L_WRIST_YAW] = lWristYaw_();
  jointCalibrationData_->calibrationOffsets[JOINTS::L_HAND] = lHand_();
  jointCalibrationData_->calibrationOffsets[JOINTS::L_HIP_YAW_PITCH] = lHipYawPitch_();
  jointCalibrationData_->calibrationOffsets[JOINTS::L_HIP_ROLL] = lHipRoll_();
  jointCalibrationData_->calibrationOffsets[JOINTS::L_HIP_PITCH] = lHipPitch_();
  jointCalibrationData_->calibrationOffsets[JOINTS::L_KNEE_PITCH] = lKneePitch_();
  jointCalibrationData_->calibrationOffsets[JOINTS::L_ANKLE_PITCH] = lAnklePitch_();
  jointCalibrationData_->calibrationOffsets[JOINTS::L_ANKLE_ROLL] = lAnkleRoll_();
  jointCalibrationData_->calibrationOffsets[JOINTS::R_HIP_YAW_PITCH] = rHipYawPitch_();
  jointCalibrationData_->calibrationOffsets[JOINTS::R_HIP_ROLL] = rHipRoll_();
  jointCalibrationData_->calibrationOffsets[JOINTS::R_HIP_PITCH] = rHipPitch_();
  jointCalibrationData_->calibrationOffsets[JOINTS::R_KNEE_PITCH] = rKneePitch_();
  jointCalibrationData_->calibrationOffsets[JOINTS::R_ANKLE_PITCH] = rAnklePitch_();
  jointCalibrationData_->calibrationOffsets[JOINTS::R_ANKLE_ROLL] = rAnkleRoll_();
  jointCalibrationData_->calibrationOffsets[JOINTS::R_SHOULDER_PITCH] = rShoulderPitch_();
  jointCalibrationData_->calibrationOffsets[JOINTS::R_SHOULDER_ROLL] = rShoulderRoll_();
  jointCalibrationData_->calibrationOffsets[JOINTS::R_ELBOW_YAW] = rElbowYaw_();
  jointCalibrationData_->calibrationOffsets[JOINTS::R_ELBOW_ROLL] = rElbowRoll_();
  jointCalibrationData_->calibrationOffsets[JOINTS::R_WRIST_YAW] = rWristYaw_();
  jointCalibrationData_->calibrationOffsets[JOINTS::R_HAND] = rHand_();

  for (auto& jointOffset : jointCalibrationData_->calibrationOffsets)
  {
    jointOffset *= TO_RAD;
  }
}
