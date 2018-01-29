#include "MirrorableMotionOutput.hpp"
#include "Modules/NaoProvider.h"

void MirrorableMotionOutput::mirrorAngles()
{
  angles = getMirroredAngles();
}

std::vector<float> MirrorableMotionOutput::getMirroredAngles() const
{
  // The mirror function requires a full vector of body angles!
  assert(angles.size() == JOINTS::JOINTS_MAX);

  std::vector<float> mirroredAngles(JOINTS::JOINTS_MAX);

  mirroredAngles[JOINTS::HEAD_YAW] = -angles[JOINTS::HEAD_YAW];
  mirroredAngles[JOINTS::HEAD_PITCH] = angles[JOINTS::HEAD_PITCH];

  mirroredAngles[JOINTS::L_SHOULDER_PITCH] = angles[JOINTS::R_SHOULDER_PITCH];
  mirroredAngles[JOINTS::L_SHOULDER_ROLL] = -angles[JOINTS::R_SHOULDER_ROLL];
  mirroredAngles[JOINTS::L_ELBOW_YAW] = -angles[JOINTS::R_ELBOW_YAW];
  mirroredAngles[JOINTS::L_ELBOW_ROLL] = -angles[JOINTS::R_ELBOW_ROLL];
  mirroredAngles[JOINTS::L_WRIST_YAW] = -angles[JOINTS::R_WRIST_YAW];
  mirroredAngles[JOINTS::L_HAND] = angles[JOINTS::R_HAND];
  mirroredAngles[JOINTS::L_HIP_YAW_PITCH] = angles[JOINTS::R_HIP_YAW_PITCH];
  mirroredAngles[JOINTS::L_HIP_ROLL] = -angles[JOINTS::R_HIP_ROLL];
  mirroredAngles[JOINTS::L_HIP_PITCH] = angles[JOINTS::R_HIP_PITCH];
  mirroredAngles[JOINTS::L_KNEE_PITCH] = angles[JOINTS::R_KNEE_PITCH];
  mirroredAngles[JOINTS::L_ANKLE_PITCH] = angles[JOINTS::R_ANKLE_PITCH];
  mirroredAngles[JOINTS::L_ANKLE_ROLL] = -angles[JOINTS::R_ANKLE_ROLL];
  /// ---
  mirroredAngles[JOINTS::R_SHOULDER_PITCH] = angles[JOINTS::L_SHOULDER_PITCH];
  mirroredAngles[JOINTS::R_SHOULDER_ROLL] = -angles[JOINTS::L_SHOULDER_ROLL];
  mirroredAngles[JOINTS::R_ELBOW_YAW] = -angles[JOINTS::L_ELBOW_YAW];
  mirroredAngles[JOINTS::R_ELBOW_ROLL] = -angles[JOINTS::L_ELBOW_ROLL];
  mirroredAngles[JOINTS::R_WRIST_YAW] = -angles[JOINTS::L_WRIST_YAW];
  mirroredAngles[JOINTS::R_HAND] = angles[JOINTS::L_HAND];
  mirroredAngles[JOINTS::R_HIP_YAW_PITCH] = angles[JOINTS::L_HIP_YAW_PITCH];
  mirroredAngles[JOINTS::R_HIP_ROLL] = -angles[JOINTS::L_HIP_ROLL];
  mirroredAngles[JOINTS::R_HIP_PITCH] = angles[JOINTS::L_HIP_PITCH];
  mirroredAngles[JOINTS::R_KNEE_PITCH] = angles[JOINTS::L_KNEE_PITCH];
  mirroredAngles[JOINTS::R_ANKLE_PITCH] = angles[JOINTS::L_ANKLE_PITCH];
  mirroredAngles[JOINTS::R_ANKLE_ROLL] = -angles[JOINTS::L_ANKLE_ROLL];

  return mirroredAngles;
}
