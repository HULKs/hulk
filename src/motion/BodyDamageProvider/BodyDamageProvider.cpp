#include "BodyDamageProvider.hpp"

BodyDamageProvider::BodyDamageProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , headYaw_(*this, "headYaw", [this]() { damageStateChanged_ = true; })
  , headPitch_(*this, "headPitch", [this]() { damageStateChanged_ = true; })
  , lShoulderPitch_(*this, "lShoulderPitch", [this]() { damageStateChanged_ = true; })
  , lShoulderRoll_(*this, "lShoulderRoll", [this]() { damageStateChanged_ = true; })
  , lElbowYaw_(*this, "lElbowYaw", [this]() { damageStateChanged_ = true; })
  , lElbowRoll_(*this, "lElbowRoll", [this]() { damageStateChanged_ = true; })
  , lWristYaw_(*this, "lWristYaw", [this]() { damageStateChanged_ = true; })
  , lHand_(*this, "lHand", [this]() { damageStateChanged_ = true; })
  , lHipYawPitch_(*this, "lHipYawPitch", [this]() { damageStateChanged_ = true; })
  , lHipRoll_(*this, "lHipRoll", [this]() { damageStateChanged_ = true; })
  , lHipPitch_(*this, "lHipPitch", [this]() { damageStateChanged_ = true; })
  , lKneePitch_(*this, "lKneePitch", [this]() { damageStateChanged_ = true; })
  , lAnklePitch_(*this, "lAnklePitch", [this]() { damageStateChanged_ = true; })
  , lAnkleRoll_(*this, "lAnkleRoll", [this]() { damageStateChanged_ = true; })
  , rHipYawPitch_(*this, "rHipYawPitch", [this]() { damageStateChanged_ = true; })
  , rHipRoll_(*this, "rHipRoll", [this]() { damageStateChanged_ = true; })
  , rHipPitch_(*this, "rHipPitch", [this]() { damageStateChanged_ = true; })
  , rKneePitch_(*this, "rKneePitch", [this]() { damageStateChanged_ = true; })
  , rAnklePitch_(*this, "rAnklePitch", [this]() { damageStateChanged_ = true; })
  , rAnkleRoll_(*this, "rAnkleRoll", [this]() { damageStateChanged_ = true; })
  , rShoulderPitch_(*this, "rShoulderPitch", [this]() { damageStateChanged_ = true; })
  , rShoulderRoll_(*this, "rShoulderRoll", [this]() { damageStateChanged_ = true; })
  , rElbowYaw_(*this, "rElbowYaw", [this]() { damageStateChanged_ = true; })
  , rElbowRoll_(*this, "rElbowRoll", [this]() { damageStateChanged_ = true; })
  , rWristYaw_(*this, "rWristYaw", [this]() { damageStateChanged_ = true; })
  , rHand_(*this, "rHand", [this]() { damageStateChanged_ = true; })
  , lFsrFL_(*this, "lFsrFL", [this]() { damageStateChanged_ = true; })
  , lFsrFR_(*this, "lFsrFR", [this]() { damageStateChanged_ = true; })
  , lFsrRL_(*this, "lFsrRL", [this]() { damageStateChanged_ = true; })
  , lFsrRR_(*this, "lFsrRR", [this]() { damageStateChanged_ = true; })
  , rFsrFL_(*this, "rFsrFL", [this]() { damageStateChanged_ = true; })
  , rFsrFR_(*this, "rFsrFR", [this]() { damageStateChanged_ = true; })
  , rFsrRL_(*this, "rFsrRL", [this]() { damageStateChanged_ = true; })
  , rFsrRR_(*this, "rFsrRR", [this]() { damageStateChanged_ = true; })
  , accelerometer_(*this, "accelerometer", [this]() { damageStateChanged_ = true; })
  , gyrometer_(*this, "gyrometer", [this]() { damageStateChanged_ = true; })
  , uSLeft_(*this, "uSLeft", [this]() { damageStateChanged_ = true; })
  , uSRight_(*this, "uSRight", [this]() { damageStateChanged_ = true; })
  , chestButton_(*this, "chestButton", [this]() { damageStateChanged_ = true; })
  , lHandTouchLeft_(*this, "lHandTouchLeft", [this]() { damageStateChanged_ = true; })
  , lHandTouchBack_(*this, "lHandTouchBack", [this]() { damageStateChanged_ = true; })
  , lHandTouchRight_(*this, "lHandTouchRight", [this]() { damageStateChanged_ = true; })
  , rHandTouchLeft(*this, "rHandTouchLef", [this]() { damageStateChanged_ = true; })
  , rHandTouchBack(*this, "rHandTouchBac", [this]() { damageStateChanged_ = true; })
  , rHandTouchRight(*this, "rHandTouchRigh", [this]() { damageStateChanged_ = true; })
  , bumperLFootLeft_(*this, "bumperLFootLeft", [this]() { damageStateChanged_ = true; })
  , bumperLFootRight_(*this, "bumperLFootRight", [this]() { damageStateChanged_ = true; })
  , bumperRFootLeft_(*this, "bumperRFootLeft", [this]() { damageStateChanged_ = true; })
  , bumperRFootRight_(*this, "bumperRFootRight", [this]() { damageStateChanged_ = true; })
  , damageStateChanged_(true)
  , bodyDamageData_(*this)
{
}

void BodyDamageProvider::cycle()
{
  if (damageStateChanged_)
  {
    updateState();
    damageStateChanged_ = false;
  }
}

void BodyDamageProvider::updateState()
{
  /// Joints
  bodyDamageData_->damagedJoints[JOINTS::HEAD_YAW] = headYaw_();
  bodyDamageData_->damagedJoints[JOINTS::HEAD_PITCH] = headPitch_();
  bodyDamageData_->damagedJoints[JOINTS::L_SHOULDER_PITCH] = lShoulderPitch_();
  bodyDamageData_->damagedJoints[JOINTS::L_SHOULDER_ROLL] = lShoulderRoll_();
  bodyDamageData_->damagedJoints[JOINTS::L_ELBOW_YAW] = lElbowYaw_();
  bodyDamageData_->damagedJoints[JOINTS::L_ELBOW_ROLL] = lElbowRoll_();
  bodyDamageData_->damagedJoints[JOINTS::L_WRIST_YAW] = lWristYaw_();
  bodyDamageData_->damagedJoints[JOINTS::L_HAND] = lHand_();
  bodyDamageData_->damagedJoints[JOINTS::L_HIP_YAW_PITCH] = lHipYawPitch_();
  bodyDamageData_->damagedJoints[JOINTS::L_HIP_ROLL] = lHipRoll_();
  bodyDamageData_->damagedJoints[JOINTS::L_HIP_PITCH] = lHipPitch_();
  bodyDamageData_->damagedJoints[JOINTS::L_KNEE_PITCH] = lKneePitch_();
  bodyDamageData_->damagedJoints[JOINTS::L_ANKLE_PITCH] = lAnklePitch_();
  bodyDamageData_->damagedJoints[JOINTS::L_ANKLE_ROLL] = lAnkleRoll_();
  bodyDamageData_->damagedJoints[JOINTS::R_HIP_YAW_PITCH] = rHipYawPitch_();
  bodyDamageData_->damagedJoints[JOINTS::R_HIP_ROLL] = rHipRoll_();
  bodyDamageData_->damagedJoints[JOINTS::R_HIP_PITCH] = rHipPitch_();
  bodyDamageData_->damagedJoints[JOINTS::R_KNEE_PITCH] = rKneePitch_();
  bodyDamageData_->damagedJoints[JOINTS::R_ANKLE_PITCH] = rAnklePitch_();
  bodyDamageData_->damagedJoints[JOINTS::R_ANKLE_ROLL] = rAnkleRoll_();
  bodyDamageData_->damagedJoints[JOINTS::R_SHOULDER_PITCH] = rShoulderPitch_();
  bodyDamageData_->damagedJoints[JOINTS::R_SHOULDER_ROLL] = rShoulderRoll_();
  bodyDamageData_->damagedJoints[JOINTS::R_ELBOW_YAW] = rElbowYaw_();
  bodyDamageData_->damagedJoints[JOINTS::R_ELBOW_ROLL] = rElbowRoll_();
  bodyDamageData_->damagedJoints[JOINTS::R_WRIST_YAW] = rWristYaw_();
  bodyDamageData_->damagedJoints[JOINTS::R_HAND] = rHand_();
  /// FSRs
  bodyDamageData_->damagedFSRs[FSRS::L_FL] = lFsrFL_();
  bodyDamageData_->damagedFSRs[FSRS::L_FR] = lFsrFR_();
  bodyDamageData_->damagedFSRs[FSRS::L_RL] = lFsrRL_();
  bodyDamageData_->damagedFSRs[FSRS::L_RR] = lFsrRR_();
  bodyDamageData_->damagedFSRs[FSRS::R_FL] = rFsrFL_();
  bodyDamageData_->damagedFSRs[FSRS::R_FR] = rFsrFR_();
  bodyDamageData_->damagedFSRs[FSRS::R_RL] = rFsrRL_();
  bodyDamageData_->damagedFSRs[FSRS::R_RR] = rFsrRR_();
  /// Inertial unit
  bodyDamageData_->damagedIMU[IMU::ACCELEROMETER] = accelerometer_();
  bodyDamageData_->damagedIMU[IMU::ACCELEROMETER] = gyrometer_();
  /// Ultra sonic sensors
  bodyDamageData_->damagedSonars[SONARS::LEFT] = uSLeft_();
  bodyDamageData_->damagedSonars[SONARS::RIGHT] = uSRight_();
  /// Contact and tactile sensors - Chest button
  bodyDamageData_->damagedButtons[BUTTONS::CHEST] = chestButton_();
  /// Contact and tactile sensors - Hand
  bodyDamageData_->damagedTactileHandSensors[TACTILEHANDSENSORS::LLEFT] = lHandTouchLeft_();
  bodyDamageData_->damagedTactileHandSensors[TACTILEHANDSENSORS::LBACK] = lHandTouchBack_();
  bodyDamageData_->damagedTactileHandSensors[TACTILEHANDSENSORS::LRIGHT] = lHandTouchRight_();
  bodyDamageData_->damagedTactileHandSensors[TACTILEHANDSENSORS::RLEFT] = rHandTouchLeft();
  bodyDamageData_->damagedTactileHandSensors[TACTILEHANDSENSORS::RBACK] = rHandTouchBack();
  bodyDamageData_->damagedTactileHandSensors[TACTILEHANDSENSORS::RRIGHT] = rHandTouchRight();
  /// Contact and tactile sensors - Foot
  bodyDamageData_->damagedBumpers[BUMPERS::LLEFT] = bumperLFootLeft_();
  bodyDamageData_->damagedBumpers[BUMPERS::LRIGHT] = bumperLFootRight_();
  bodyDamageData_->damagedBumpers[BUMPERS::RLEFT] = bumperRFootLeft_();
  bodyDamageData_->damagedBumpers[BUMPERS::RRIGHT] = bumperRFootRight_();
}
