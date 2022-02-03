#include "Hardware/Kinematics/ForwardKinematics.hpp"
#include "Hardware/JointUtils.hpp"
#include "Hardware/RobotMetrics.hpp"
#include "Tools/Math/Angle.hpp"

ForwardKinematics::ForwardKinematics(const RobotMetrics& robotMetrics)
  : robotMetrics_(robotMetrics)
{
}

KinematicMatrix ForwardKinematics::getHeadYaw(const JointsHeadArray<float>& jointAngles) const
{
  KinematicMatrix headYaw2Torso =
      KinematicMatrix::transZ(robotMetrics_.link(Links::NECK_OFFSET_Z)) *
      KinematicMatrix::rotZ(jointAngles[JointsHead::YAW]);
  return headYaw2Torso;
}

KinematicMatrix ForwardKinematics::getHeadPitch(const JointsHeadArray<float>& jointAngles) const
{
  KinematicMatrix headPitch2HeadYaw = KinematicMatrix::rotY(jointAngles[JointsHead::PITCH]);
  return getHeadYaw(jointAngles) * headPitch2HeadYaw;
}


KinematicMatrix ForwardKinematics::getLShoulderPitch(const JointsArmArray<float>& jointAngles) const
{
  KinematicMatrix lShoulderBase2Torso =
      KinematicMatrix::transZ(robotMetrics_.link(Links::SHOULDER_OFFSET_Z)) *
      KinematicMatrix::transY(robotMetrics_.link(Links::SHOULDER_OFFSET_Y));

  KinematicMatrix lShoulderPitch2LShoulderBase =
      KinematicMatrix::rotY(jointAngles[JointsArm::SHOULDER_PITCH]);

  return lShoulderBase2Torso * lShoulderPitch2LShoulderBase;
}

KinematicMatrix ForwardKinematics::getLShoulderRoll(const JointsArmArray<float>& jointAngles) const
{
  KinematicMatrix lShoulderRoll2LShoulderPitch =
      KinematicMatrix::rotZ(jointAngles[JointsArm::SHOULDER_ROLL]);

  return getLShoulderPitch(jointAngles) * lShoulderRoll2LShoulderPitch;
}

KinematicMatrix ForwardKinematics::getLElbowYaw(const JointsArmArray<float>& jointAngles) const
{
  KinematicMatrix lElbowYaw2LShoulderRoll =
      KinematicMatrix::transX(robotMetrics_.link(Links::UPPER_ARM_LENGTH)) *
      KinematicMatrix::transY(robotMetrics_.link(Links::ELBOW_OFFSET_Y)) *
      KinematicMatrix::rotX(jointAngles[JointsArm::ELBOW_YAW]);

  return getLShoulderRoll(jointAngles) * lElbowYaw2LShoulderRoll;
}

KinematicMatrix ForwardKinematics::getLElbowRoll(const JointsArmArray<float>& jointAngles) const
{
  KinematicMatrix lElbowRoll2LElbowYaw = KinematicMatrix::rotZ(jointAngles[JointsArm::ELBOW_ROLL]);

  return getLElbowYaw(jointAngles) * lElbowRoll2LElbowYaw;
}

KinematicMatrix ForwardKinematics::getLWristYaw(const JointsArmArray<float>& jointAngles) const
{
  KinematicMatrix lWristYaw2LElbowRoll =
      KinematicMatrix::transX(robotMetrics_.link(Links::LOWER_ARM_LENGTH)) *
      KinematicMatrix::rotX(jointAngles[JointsArm::WRIST_YAW]);

  return getLElbowRoll(jointAngles) * lWristYaw2LElbowRoll;
}

KinematicMatrix ForwardKinematics::getLHand(const JointsArmArray<float>& jointAngles) const
{
  KinematicMatrix lHand2LWristYaw =
      KinematicMatrix::transX(robotMetrics_.link(Links::HAND_OFFSET_X)); // *
  // KinematicMatrix::transZ(NaoProvider::link(LINKS::HAND_OFFSET_Z)) *
  // KinematicMatrix::rotY( jointAngles.at(5) );

  return getLWristYaw(jointAngles) * lHand2LWristYaw;
}

KinematicMatrix ForwardKinematics::getRShoulderPitch(const JointsArmArray<float>& jointAngles) const
{
  KinematicMatrix rShoulderBase2Torso =
      KinematicMatrix::transZ(robotMetrics_.link(Links::SHOULDER_OFFSET_Z)) *
      KinematicMatrix::transY(-robotMetrics_.link(Links::SHOULDER_OFFSET_Y));

  KinematicMatrix rShoulderPitch2RShoulderBase =
      KinematicMatrix::rotY(jointAngles[JointsArm::SHOULDER_PITCH]);

  return rShoulderBase2Torso * rShoulderPitch2RShoulderBase;
}

KinematicMatrix ForwardKinematics::getRShoulderRoll(const JointsArmArray<float>& jointAngles) const
{
  KinematicMatrix rShoulderRoll2RShoulderPitch =
      KinematicMatrix::rotZ(jointAngles[JointsArm::SHOULDER_ROLL]);

  return getRShoulderPitch(jointAngles) * rShoulderRoll2RShoulderPitch;
}

KinematicMatrix ForwardKinematics::getRElbowYaw(const JointsArmArray<float>& jointAngles) const
{
  KinematicMatrix rElbowYaw2RShoulderRoll =
      KinematicMatrix::transX(robotMetrics_.link(Links::UPPER_ARM_LENGTH)) *
      KinematicMatrix::transY(-robotMetrics_.link(Links::ELBOW_OFFSET_Y)) *
      KinematicMatrix::rotX(jointAngles[JointsArm::ELBOW_YAW]);

  return getRShoulderRoll(jointAngles) * rElbowYaw2RShoulderRoll;
}

KinematicMatrix ForwardKinematics::getRElbowRoll(const JointsArmArray<float>& jointAngles) const
{
  KinematicMatrix rElbowRoll2RElbowYaw = KinematicMatrix::rotZ(jointAngles[JointsArm::ELBOW_ROLL]);

  return getRElbowYaw(jointAngles) * rElbowRoll2RElbowYaw;
}

KinematicMatrix ForwardKinematics::getRWristYaw(const JointsArmArray<float>& jointAngles) const
{
  KinematicMatrix rWristYaw2RElbowRoll =
      KinematicMatrix::transX(robotMetrics_.link(Links::LOWER_ARM_LENGTH)) *
      KinematicMatrix::rotX(jointAngles[JointsArm::WRIST_YAW]);

  return getRElbowRoll(jointAngles) * rWristYaw2RElbowRoll;
}

KinematicMatrix ForwardKinematics::getRHand(const JointsArmArray<float>& jointAngles) const
{
  KinematicMatrix rHand2RWristYaw =
      KinematicMatrix::transX(robotMetrics_.link(Links::HAND_OFFSET_X)); // *
  // KinematicMatrix::transZ(NaoProvider::link(LINKS::HAND_OFFSET_Z)) *
  // KinematicMatrix::rotY( jointAngles.at(5) );

  return getRWristYaw(jointAngles) * rHand2RWristYaw;
}


KinematicMatrix ForwardKinematics::getLHipYawPitch(const JointsLegArray<float>& jointAngles) const
{
  // From Torso to Hip
  KinematicMatrix lHipBase2Torso =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::HIP_OFFSET_Z)) *
      KinematicMatrix::transY(robotMetrics_.link(Links::HIP_OFFSET_Y));

  // From Hip to rotated Hip
  KinematicMatrix lHipYawPitch2LHipBase =
      KinematicMatrix::rotX(-45.0f * TO_RAD) *
      KinematicMatrix::rotY(jointAngles[JointsLeg::HIP_YAW_PITCH]);

  // From rotated Hip to Torso
  return lHipBase2Torso * lHipYawPitch2LHipBase;
}

KinematicMatrix ForwardKinematics::getLHipRoll(const JointsLegArray<float>& jointAngles) const
{
  // From LHipRoll to LHipYawPitch
  KinematicMatrix lHipRoll2LHipYawPitch =
      KinematicMatrix::rotX(45.0f * TO_RAD + jointAngles[JointsLeg::HIP_ROLL]);

  // From LHipRoll to Torso
  return getLHipYawPitch(jointAngles) * lHipRoll2LHipYawPitch;
}

KinematicMatrix ForwardKinematics::getLHipPitch(const JointsLegArray<float>& jointAngles) const
{
  // From LHipPitch to LHipRoll
  KinematicMatrix lHipPitch2LHipRoll = KinematicMatrix::rotY(jointAngles[JointsLeg::HIP_PITCH]);

  // From LHipPitch to Torso
  return getLHipRoll(jointAngles) * lHipPitch2LHipRoll;
}

KinematicMatrix ForwardKinematics::getLKneePitch(const JointsLegArray<float>& jointAngles) const
{
  // From LKneePitch to LHipPitch
  KinematicMatrix lKneePitch2LHipPitch =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::THIGH_LENGTH)) *
      KinematicMatrix::rotY(jointAngles[JointsLeg::KNEE_PITCH]);

  // From LKneePitch to Torso
  return getLHipPitch(jointAngles) * lKneePitch2LHipPitch;
}

KinematicMatrix ForwardKinematics::getLAnklePitch(const JointsLegArray<float>& jointAngles) const
{
  // From LAnklePitch to LKneePitch
  KinematicMatrix lAnklePitch2LKneePitch =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::TIBIA_LENGTH)) *
      KinematicMatrix::rotY(jointAngles[JointsLeg::ANKLE_PITCH]);

  // From LAnklePitch to Torso
  return getLKneePitch(jointAngles) * lAnklePitch2LKneePitch;
}

KinematicMatrix ForwardKinematics::getLAnkleRoll(const JointsLegArray<float>& jointAngles) const
{
  // From LAnkleRoll to LAnklePitch
  KinematicMatrix lAnkleRoll2LAnklePitch =
      KinematicMatrix::rotX(jointAngles[JointsLeg::ANKLE_ROLL]);

  // From LAnkleRoll to Torso
  return getLAnklePitch(jointAngles) * lAnkleRoll2LAnklePitch;
}

KinematicMatrix ForwardKinematics::getLFoot(const JointsLegArray<float>& jointAngles) const
{
  // From LFoot to LAnkleRoll
  KinematicMatrix lFoot2LAnkleRoll =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::FOOT_HEIGHT));

  // From LFoot to Torso
  return getLAnkleRoll(jointAngles) * lFoot2LAnkleRoll;
}

KinematicMatrix ForwardKinematics::getRHipYawPitch(const JointsLegArray<float>& jointAngles) const
{
  // From Torso to Hip
  KinematicMatrix rHipBase2Torso =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::HIP_OFFSET_Z)) *
      KinematicMatrix::transY(-robotMetrics_.link(Links::HIP_OFFSET_Y));

  // From Hip to rotated Hip
  KinematicMatrix rHipYawPitch2LHipBase =
      KinematicMatrix::rotX(-135 * TO_RAD) *
      KinematicMatrix::rotY(-jointAngles[JointsLeg::HIP_YAW_PITCH]);

  // From rotated Hip to Torso
  return rHipBase2Torso * rHipYawPitch2LHipBase;
}

KinematicMatrix ForwardKinematics::getRHipRoll(const JointsLegArray<float>& jointAngles) const
{
  // From LHipRoll to LHipYawPitch
  KinematicMatrix rHipRoll2RHipYawPitch =
      KinematicMatrix::rotX(135 * TO_RAD + jointAngles[JointsLeg::HIP_ROLL]);

  // From LHipRoll to Torso
  return getRHipYawPitch(jointAngles) * rHipRoll2RHipYawPitch;
}

KinematicMatrix ForwardKinematics::getRHipPitch(const JointsLegArray<float>& jointAngles) const
{
  // From RHipPitch to RHipRoll
  KinematicMatrix rHipPitch2RHipRoll = KinematicMatrix::rotY(jointAngles[JointsLeg::HIP_PITCH]);

  // From LHipPitch to Torso
  return getRHipRoll(jointAngles) * rHipPitch2RHipRoll;
}

KinematicMatrix ForwardKinematics::getRKneePitch(const JointsLegArray<float>& jointAngles) const
{
  // From RKneePitch to RHipPitch
  KinematicMatrix rKneePitch2RHipPitch =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::THIGH_LENGTH)) *
      KinematicMatrix::rotY(jointAngles[JointsLeg::KNEE_PITCH]);

  // From RKneePitch to Torso
  return getRHipPitch(jointAngles) * rKneePitch2RHipPitch;
}

KinematicMatrix ForwardKinematics::getRAnklePitch(const JointsLegArray<float>& jointAngles) const
{
  // From RAnklePitch to RKneePitch
  KinematicMatrix rAnklePitch2RKneePitch =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::TIBIA_LENGTH)) *
      KinematicMatrix::rotY(jointAngles[JointsLeg::ANKLE_PITCH]);

  // From RAnklePitch to Torso
  return getRKneePitch(jointAngles) * rAnklePitch2RKneePitch;
}

KinematicMatrix ForwardKinematics::getRAnkleRoll(const JointsLegArray<float>& jointAngles) const
{
  // From RAnkleRoll to RAnklePitch
  KinematicMatrix rAnkleRoll2RAnklePitch =
      KinematicMatrix::rotX(jointAngles[JointsLeg::ANKLE_ROLL]);

  // From RAnkleRoll to Torso
  return getRAnklePitch(jointAngles) * rAnkleRoll2RAnklePitch;
}

KinematicMatrix ForwardKinematics::getRFoot(const JointsLegArray<float>& jointAngles) const
{
  // From RFoot to RAnkleRoll
  KinematicMatrix rFoot2RAnkleRoll =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::FOOT_HEIGHT));

  // From LFoot to Torso
  return getRAnkleRoll(jointAngles) * rFoot2RAnkleRoll;
}

JointsHeadArray<KinematicMatrix>
ForwardKinematics::getHead(const JointsHeadArray<float>& jointAngles) const
{
  KinematicMatrix headYaw2Torso =
      KinematicMatrix::transZ(robotMetrics_.link(Links::NECK_OFFSET_Z)) *
      KinematicMatrix::rotZ(jointAngles[JointsHead::YAW]);

  KinematicMatrix headPitch2HeadYaw = KinematicMatrix::rotY(jointAngles[JointsHead::PITCH]);

  KinematicMatrix headPitch2Torso = headYaw2Torso * headPitch2HeadYaw;

  return {{headYaw2Torso, headPitch2Torso}};
}

JointsArmArray<KinematicMatrix>
ForwardKinematics::getLArm(const JointsArmArray<float>& jointAngles) const
{
  KinematicMatrix lShoulderBase2Torso =
      KinematicMatrix::transZ(robotMetrics_.link(Links::SHOULDER_OFFSET_Z)) *
      KinematicMatrix::transY(robotMetrics_.link(Links::SHOULDER_OFFSET_Y));

  KinematicMatrix lShoulderPitch2LShoulderBase =
      KinematicMatrix::rotY(jointAngles[JointsArm::SHOULDER_PITCH]);

  // Left Shoulder Pitch
  KinematicMatrix lShoulderPitch2Torso = lShoulderBase2Torso * lShoulderPitch2LShoulderBase;

  KinematicMatrix lShoulderRoll2LShoulderPitch =
      KinematicMatrix::rotZ(jointAngles[JointsArm::SHOULDER_ROLL]);

  // Left Shoulder Roll
  KinematicMatrix lShoulderRoll2Torso = lShoulderPitch2Torso * lShoulderRoll2LShoulderPitch;

  KinematicMatrix lElbowYaw2LShoulderRoll =
      KinematicMatrix::transX(robotMetrics_.link(Links::UPPER_ARM_LENGTH)) *
      KinematicMatrix::transY(robotMetrics_.link(Links::ELBOW_OFFSET_Y)) *
      KinematicMatrix::rotX(jointAngles[JointsArm::ELBOW_YAW]);

  // Left Elbow Yaw
  KinematicMatrix lElbowYaw2Torso = lShoulderRoll2Torso * lElbowYaw2LShoulderRoll;


  KinematicMatrix lElbowRoll2LElbowYaw = KinematicMatrix::rotZ(jointAngles[JointsArm::ELBOW_ROLL]);

  // Left Elbow Roll
  KinematicMatrix lElbowRoll2Torso = lElbowYaw2Torso * lElbowRoll2LElbowYaw;

  KinematicMatrix lWristYaw2LElbowRoll =
      KinematicMatrix::transX(robotMetrics_.link(Links::LOWER_ARM_LENGTH)) *
      KinematicMatrix::rotX(jointAngles[JointsArm::WRIST_YAW]);

  // Left Wrist Yaw
  KinematicMatrix lWrist2Torso = lElbowRoll2Torso * lWristYaw2LElbowRoll;

  KinematicMatrix lHand2LWristYaw =
      KinematicMatrix::transX(robotMetrics_.link(Links::HAND_OFFSET_X)); //*
  // KinematicMatrix::transZ(robotMetrics_.link(Links::HAND_OFFSET_Z)) *
  // KinematicMatrix::rotY( jointAngles.at(5) );

  KinematicMatrix lHand2Torso = lWrist2Torso * lHand2LWristYaw;

  return {{lShoulderPitch2Torso, lShoulderRoll2Torso, lElbowYaw2Torso, lElbowRoll2Torso,
           lWrist2Torso, lHand2Torso}};
}

JointsArmArray<KinematicMatrix>
ForwardKinematics::getRArm(const JointsArmArray<float>& jointAngles) const
{
  KinematicMatrix rShoulderBase2Torso =
      KinematicMatrix::transZ(robotMetrics_.link(Links::SHOULDER_OFFSET_Z)) *
      KinematicMatrix::transY(-robotMetrics_.link(Links::SHOULDER_OFFSET_Y));

  KinematicMatrix rShoulderPitch2RShoulderBase =
      KinematicMatrix::rotY(jointAngles[JointsArm::SHOULDER_PITCH]);

  // Right Shoulder Pitch
  KinematicMatrix rShoulderPitch2Torso = rShoulderBase2Torso * rShoulderPitch2RShoulderBase;

  KinematicMatrix rShoulderRoll2RShoulderPitch =
      KinematicMatrix::rotZ(jointAngles[JointsArm::SHOULDER_ROLL]);

  // Right Shoulder Roll
  KinematicMatrix rShoulderRoll2Torso = rShoulderPitch2Torso * rShoulderRoll2RShoulderPitch;

  KinematicMatrix rElbowYaw2RShoulderRoll =
      KinematicMatrix::transX(robotMetrics_.link(Links::UPPER_ARM_LENGTH)) *
      KinematicMatrix::transY(-robotMetrics_.link(Links::ELBOW_OFFSET_Y)) *
      KinematicMatrix::rotX(jointAngles[JointsArm::ELBOW_YAW]);

  // Right Elbow Yaw
  KinematicMatrix rElbowYaw2Torso = rShoulderRoll2Torso * rElbowYaw2RShoulderRoll;

  KinematicMatrix rElbowRoll2RElbowYaw = KinematicMatrix::rotZ(jointAngles[JointsArm::ELBOW_ROLL]);

  // Right Elbow Roll
  KinematicMatrix rElbowRoll2Torso = rElbowYaw2Torso * rElbowRoll2RElbowYaw;

  KinematicMatrix rWristYaw2RElbowRoll =
      KinematicMatrix::transX(robotMetrics_.link(Links::LOWER_ARM_LENGTH)) *
      KinematicMatrix::rotX(jointAngles[JointsArm::WRIST_YAW]);

  // Right Wrist Yaw
  KinematicMatrix rWristYaw2Torso = rElbowRoll2Torso * rWristYaw2RElbowRoll;

  KinematicMatrix rHand2RWristYaw =
      KinematicMatrix::transX(robotMetrics_.link(Links::HAND_OFFSET_X)); // *
  // KinematicMatrix::transZ(robotMetrics_.link(Links::HAND_OFFSET_Z)) *
  // KinematicMatrix::rotY( jointAngles.at(5) );

  KinematicMatrix rHand2Torso = rWristYaw2Torso * rHand2RWristYaw;

  return {{rShoulderPitch2Torso, rShoulderRoll2Torso, rElbowYaw2Torso, rElbowRoll2Torso,
           rWristYaw2Torso, rHand2Torso}};
}

JointsLegArray<KinematicMatrix>
ForwardKinematics::getLLeg(const JointsLegArray<float>& jointAngles) const
{
  // From Torso to Hip
  KinematicMatrix lHipBase2Torso =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::HIP_OFFSET_Z)) *
      KinematicMatrix::transY(robotMetrics_.link(Links::HIP_OFFSET_Y));

  // From Hip to rotated Hip
  KinematicMatrix lHipYawPitch2LHipBase =
      KinematicMatrix::rotX(-45.0f * TO_RAD) *
      KinematicMatrix::rotY(jointAngles[JointsLeg::HIP_YAW_PITCH]);

  // From rotated Hip to Torso
  KinematicMatrix lHipYawPitch2Torso = lHipBase2Torso * lHipYawPitch2LHipBase;

  // From LHipRoll to LHipYawPitch
  KinematicMatrix lHipRoll2LHipYawPitch =
      KinematicMatrix::rotX(45.0f * TO_RAD + jointAngles[JointsLeg::HIP_ROLL]);

  // From LHipRoll to Torso
  KinematicMatrix lHipRoll2Torso = lHipYawPitch2Torso * lHipRoll2LHipYawPitch;

  // From LHipPitch to LHipRoll
  KinematicMatrix lHipPitch2LHipRoll = KinematicMatrix::rotY(jointAngles[JointsLeg::HIP_PITCH]);

  // From LHipPitch to Torso
  KinematicMatrix lHipPitch2Torso = lHipRoll2Torso * lHipPitch2LHipRoll;

  // From LKneePitch to LHipPitch
  KinematicMatrix lKneePitch2LHipPitch =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::THIGH_LENGTH)) *
      KinematicMatrix::rotY(jointAngles[JointsLeg::KNEE_PITCH]);

  // From LKneePitch to Torso
  KinematicMatrix lKneePitch2Torso = lHipPitch2Torso * lKneePitch2LHipPitch;

  // From LAnklePitch to LKneePitch
  KinematicMatrix lAnklePitch2LKneePitch =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::TIBIA_LENGTH)) *
      KinematicMatrix::rotY(jointAngles[JointsLeg::ANKLE_PITCH]);

  // From LAnklePitch to Torso
  KinematicMatrix lAnklePitch2Torso = lKneePitch2Torso * lAnklePitch2LKneePitch;

  // From LAnkleRoll to LAnklePitch
  KinematicMatrix lAnkleRoll2LAnklePitch =
      KinematicMatrix::rotX(jointAngles[JointsLeg::ANKLE_ROLL]);

  // From LAnkleRoll to Torso
  KinematicMatrix lAnkleRoll2Torso = lAnklePitch2Torso * lAnkleRoll2LAnklePitch;

  // From LFoot to LAnkleRoll
  KinematicMatrix lFoot2LAnkleRoll =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::FOOT_HEIGHT));

  return {{lHipYawPitch2Torso, lHipRoll2Torso, lHipPitch2Torso, lKneePitch2Torso, lAnklePitch2Torso,
           lAnkleRoll2Torso}};
}

JointsLegArray<KinematicMatrix>
ForwardKinematics::getRLeg(const JointsLegArray<float>& jointAngles) const
{
  // From Torso to Hip
  KinematicMatrix rHipBase2Torso =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::HIP_OFFSET_Z)) *
      KinematicMatrix::transY(-robotMetrics_.link(Links::HIP_OFFSET_Y));

  // From Hip to rotated Hip
  KinematicMatrix rHipYawPitch2LHipBase =
      KinematicMatrix::rotX(-135 * TO_RAD) *
      KinematicMatrix::rotY(-jointAngles[JointsLeg::HIP_YAW_PITCH]);

  // From rotated Hip to Torso
  KinematicMatrix rHipYawPitch2Torso = rHipBase2Torso * rHipYawPitch2LHipBase;

  // From LHipRoll to LHipYawPitch
  KinematicMatrix rHipRoll2RHipYawPitch =
      KinematicMatrix::rotX(135 * TO_RAD + jointAngles[JointsLeg::HIP_ROLL]);

  // From LHipRoll to Torso
  KinematicMatrix rHipRoll2Torso = rHipYawPitch2Torso * rHipRoll2RHipYawPitch;

  // From RHipPitch to RHipRoll
  KinematicMatrix rHipPitch2RHipRoll = KinematicMatrix::rotY(jointAngles[JointsLeg::HIP_PITCH]);

  // From LHipPitch to Torso
  KinematicMatrix rHipPitch2Torso = rHipRoll2Torso * rHipPitch2RHipRoll;

  // From RKneePitch to RHipPitch
  KinematicMatrix rKneePitch2RHipPitch =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::THIGH_LENGTH)) *
      KinematicMatrix::rotY(jointAngles[JointsLeg::KNEE_PITCH]);

  // From RKneePitch to Torso
  KinematicMatrix rKneePitch2Torso = rHipPitch2Torso * rKneePitch2RHipPitch;

  // From RAnklePitch to RKneePitch
  KinematicMatrix rAnklePitch2RKneePitch =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::TIBIA_LENGTH)) *
      KinematicMatrix::rotY(jointAngles[JointsLeg::ANKLE_PITCH]);

  // From RAnklePitch to Torso
  KinematicMatrix rAnklePitch2Torso = rKneePitch2Torso * rAnklePitch2RKneePitch;

  // From RAnkleRoll to RAnklePitch
  KinematicMatrix rAnkleRoll2RAnklePitch =
      KinematicMatrix::rotX(jointAngles[JointsLeg::ANKLE_ROLL]);

  // From RAnkleRoll to Torso
  KinematicMatrix rAnkleRoll2Torso = rAnklePitch2Torso * rAnkleRoll2RAnklePitch;

  // From RFoot to RAnkleRoll
  KinematicMatrix rFoot2RAnkleRoll =
      KinematicMatrix::transZ(-robotMetrics_.link(Links::FOOT_HEIGHT));

  return {{rHipYawPitch2Torso, rHipRoll2Torso, rHipPitch2Torso, rKneePitch2Torso, rAnklePitch2Torso,
           rAnkleRoll2Torso}};
}

JointsArray<KinematicMatrix> ForwardKinematics::getBody(const JointsArray<float>& jointAngles) const
{
  const auto headAngles = JointUtils::extractHead(jointAngles);
  const auto lLegAngles = JointUtils::extractLeftLeg(jointAngles);
  const auto rLegAngles = JointUtils::extractRightLeg(jointAngles);
  const auto lArmAngles = JointUtils::extractLeftArm(jointAngles);
  const auto rArmAngles = JointUtils::extractRightArm(jointAngles);

  const auto headKin = getHead(headAngles);
  const auto lArmKin = getLArm(lArmAngles);
  const auto lLegKin = getLLeg(lLegAngles);
  const auto rLegKin = getRLeg(rLegAngles);
  const auto rArmKin = getRArm(rArmAngles);

  JointsArray<KinematicMatrix> out;
  JointUtils::fillHead(out, headKin);
  JointUtils::fillArms(out, lArmKin, rArmKin);
  JointUtils::fillLegs(out, lLegKin, rLegKin);
  return out;
}
