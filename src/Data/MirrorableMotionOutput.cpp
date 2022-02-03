#include "Data/MirrorableMotionOutput.hpp"

void MirrorableMotionOutput::mirrorAngles()
{
  angles = getMirroredAngles();
}

JointsArray<float> MirrorableMotionOutput::getMirroredAngles() const
{
  JointsArray<float> mirroredAngles;

  mirroredAngles[Joints::HEAD_YAW] = -angles[Joints::HEAD_YAW];
  mirroredAngles[Joints::HEAD_PITCH] = angles[Joints::HEAD_PITCH];

  mirroredAngles[Joints::L_SHOULDER_PITCH] = angles[Joints::R_SHOULDER_PITCH];
  mirroredAngles[Joints::L_SHOULDER_ROLL] = -angles[Joints::R_SHOULDER_ROLL];
  mirroredAngles[Joints::L_ELBOW_YAW] = -angles[Joints::R_ELBOW_YAW];
  mirroredAngles[Joints::L_ELBOW_ROLL] = -angles[Joints::R_ELBOW_ROLL];
  mirroredAngles[Joints::L_WRIST_YAW] = -angles[Joints::R_WRIST_YAW];
  mirroredAngles[Joints::L_HAND] = angles[Joints::R_HAND];
  mirroredAngles[Joints::L_HIP_YAW_PITCH] = angles[Joints::R_HIP_YAW_PITCH];
  mirroredAngles[Joints::L_HIP_ROLL] = -angles[Joints::R_HIP_ROLL];
  mirroredAngles[Joints::L_HIP_PITCH] = angles[Joints::R_HIP_PITCH];
  mirroredAngles[Joints::L_KNEE_PITCH] = angles[Joints::R_KNEE_PITCH];
  mirroredAngles[Joints::L_ANKLE_PITCH] = angles[Joints::R_ANKLE_PITCH];
  mirroredAngles[Joints::L_ANKLE_ROLL] = -angles[Joints::R_ANKLE_ROLL];
  /// ---
  mirroredAngles[Joints::R_SHOULDER_PITCH] = angles[Joints::L_SHOULDER_PITCH];
  mirroredAngles[Joints::R_SHOULDER_ROLL] = -angles[Joints::L_SHOULDER_ROLL];
  mirroredAngles[Joints::R_ELBOW_YAW] = -angles[Joints::L_ELBOW_YAW];
  mirroredAngles[Joints::R_ELBOW_ROLL] = -angles[Joints::L_ELBOW_ROLL];
  mirroredAngles[Joints::R_WRIST_YAW] = -angles[Joints::L_WRIST_YAW];
  mirroredAngles[Joints::R_HAND] = angles[Joints::L_HAND];
  mirroredAngles[Joints::R_HIP_YAW_PITCH] = angles[Joints::L_HIP_YAW_PITCH];
  mirroredAngles[Joints::R_HIP_ROLL] = -angles[Joints::L_HIP_ROLL];
  mirroredAngles[Joints::R_HIP_PITCH] = angles[Joints::L_HIP_PITCH];
  mirroredAngles[Joints::R_KNEE_PITCH] = angles[Joints::L_KNEE_PITCH];
  mirroredAngles[Joints::R_ANKLE_PITCH] = angles[Joints::L_ANKLE_PITCH];
  mirroredAngles[Joints::R_ANKLE_ROLL] = -angles[Joints::L_ANKLE_ROLL];

  return mirroredAngles;
}
