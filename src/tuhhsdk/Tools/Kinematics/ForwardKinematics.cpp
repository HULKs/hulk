//
#include "ForwardKinematics.h"
#include <Modules/NaoProvider.h>

using namespace LINKS;
using namespace std;


/*  +----------+
 *  |   Head   |
 *  +----------+
 */

//HeadYaw
KinematicMatrix ForwardKinematics::getHeadYaw(const vector<float>& jointAngles)
{
  KinematicMatrix HeadYaw2Torso =
      KinematicMatrix::transZ(NaoProvider::link(NECK_OFFSET_Z)) *
      KinematicMatrix::rotZ(jointAngles[JOINTS_HEAD::HEAD_YAW]);

  return HeadYaw2Torso;
}

// HeadPitch
KinematicMatrix ForwardKinematics::getHeadPitch(const vector<float>& jointAngles)
{
  KinematicMatrix HeadPitch2HeadYaw =
      KinematicMatrix::rotY(jointAngles[JOINTS_HEAD::HEAD_PITCH]);

  return getHeadYaw( jointAngles) * HeadPitch2HeadYaw;
}


/*  +----------+
 *  | Left Arm |
 *  +----------+
 */

// Left Shoulder Pitch
KinematicMatrix ForwardKinematics::getLShoulderPitch(const vector<float>& jointAngles)
{
  KinematicMatrix LShoulderBase2Torso =
      KinematicMatrix::transZ(NaoProvider::link(SHOULDER_OFFSET_Z)) *
      KinematicMatrix::transY(NaoProvider::link(SHOULDER_OFFSET_Y));

  KinematicMatrix LShoulderPitch2LShoulderBase =
      KinematicMatrix::rotY(jointAngles[JOINTS_L_ARM::L_SHOULDER_PITCH]);

  return LShoulderBase2Torso * LShoulderPitch2LShoulderBase;
}

// Left Shoulder Roll
KinematicMatrix ForwardKinematics::getLShoulderRoll(const vector<float>& jointAngles)
{
  KinematicMatrix LShoulderRoll2LShoulderPitch =
      KinematicMatrix::rotZ(jointAngles[JOINTS_L_ARM::L_SHOULDER_ROLL]);

  return getLShoulderPitch( jointAngles ) * LShoulderRoll2LShoulderPitch;
}

// Left Elbow Yaw
KinematicMatrix ForwardKinematics::getLElbowYaw(const vector<float>& jointAngles)
{
  KinematicMatrix LElbowYaw2LShoulderRoll =
      KinematicMatrix::transX(NaoProvider::link(UPPER_ARM_LENGTH)) *
      KinematicMatrix::transY(NaoProvider::link(ELBOW_OFFSET_Y)) *
      KinematicMatrix::rotX( jointAngles[JOINTS_L_ARM::L_ELBOW_YAW] );

  return getLShoulderRoll( jointAngles ) * LElbowYaw2LShoulderRoll;
}

// Left Elbow Roll
KinematicMatrix ForwardKinematics::getLElbowRoll(const vector<float>& jointAngles)
{
  KinematicMatrix LElbowRoll2LElbowYaw =
      KinematicMatrix::rotZ( jointAngles[JOINTS_L_ARM::L_ELBOW_ROLL] );

  return getLElbowYaw( jointAngles ) * LElbowRoll2LElbowYaw;
}

// Left Wrist Yaw
KinematicMatrix ForwardKinematics::getLWristYaw(const vector<float>& jointAngles)
{
  KinematicMatrix LWristYaw2LElbowRoll =
      KinematicMatrix::transX(NaoProvider::link(LOWER_ARM_LENGTH)) *
      KinematicMatrix::rotX( jointAngles[JOINTS_L_ARM::L_WRIST_YAW] );

  return getLElbowRoll( jointAngles ) * LWristYaw2LElbowRoll;
}

// Left Hand
KinematicMatrix ForwardKinematics::getLHand(const vector<float>& jointAngles)
{
  KinematicMatrix LHand2LWristYaw =
      KinematicMatrix::transX(NaoProvider::link(HAND_OFFSET_X));// *
  //KinematicMatrix::transZ(NaoProvider::link(LINKS::HAND_OFFSET_Z)) *
  //KinematicMatrix::rotY( jointAngles.at(5) );

  return getLWristYaw( jointAngles ) * LHand2LWristYaw;
}

/*  +-----------+
 *  | Right Arm |
 *  +-----------+
 */

// Right Shoulder Pitch
KinematicMatrix ForwardKinematics::getRShoulderPitch(const vector<float>& jointAngles)
{
  KinematicMatrix RShoulderBase2Torso =
      KinematicMatrix::transZ(NaoProvider::link(SHOULDER_OFFSET_Z)) *
      KinematicMatrix::transY(-NaoProvider::link(SHOULDER_OFFSET_Y));

  KinematicMatrix RShoulderPitch2RShoulderBase =
      KinematicMatrix::rotY(jointAngles[JOINTS_R_ARM::R_SHOULDER_PITCH]);

  return RShoulderBase2Torso * RShoulderPitch2RShoulderBase;
}

// Right Shoulder Roll
KinematicMatrix ForwardKinematics::getRShoulderRoll(const vector<float>& jointAngles)
{
  KinematicMatrix RShoulderRoll2RShoulderPitch =
      KinematicMatrix::rotZ(jointAngles[JOINTS_R_ARM::R_SHOULDER_ROLL]);

  return getRShoulderPitch( jointAngles ) * RShoulderRoll2RShoulderPitch;
}

// Right Elbow Yaw
KinematicMatrix ForwardKinematics::getRElbowYaw(const vector<float>& jointAngles)
{
  KinematicMatrix RElbowYaw2RShoulderRoll =
      KinematicMatrix::transX(NaoProvider::link(UPPER_ARM_LENGTH)) *
      KinematicMatrix::transY(-NaoProvider::link(ELBOW_OFFSET_Y)) *
      KinematicMatrix::rotX( jointAngles[JOINTS_R_ARM::R_ELBOW_YAW] );

  return getRShoulderRoll( jointAngles ) * RElbowYaw2RShoulderRoll;
}

// Right Elbow Roll
KinematicMatrix ForwardKinematics::getRElbowRoll(const vector<float>& jointAngles)
{
  KinematicMatrix RElbowRoll2RElbowYaw =
      KinematicMatrix::rotZ( jointAngles[JOINTS_R_ARM::R_ELBOW_ROLL] );

  return getRElbowYaw( jointAngles ) * RElbowRoll2RElbowYaw;
}

// Left Wrist Yaw
KinematicMatrix ForwardKinematics::getRWristYaw(const vector<float>& jointAngles)
{
  KinematicMatrix RWristYaw2RElbowRoll =
      KinematicMatrix::transX(NaoProvider::link(LOWER_ARM_LENGTH)) *
      KinematicMatrix::rotX( jointAngles[JOINTS_R_ARM::R_WRIST_YAW] );

  return getRElbowRoll( jointAngles ) * RWristYaw2RElbowRoll;
}

// Right Hand
KinematicMatrix ForwardKinematics::getRHand(const vector<float>& jointAngles)
{
  KinematicMatrix RHand2RWristYaw =
      KinematicMatrix::transX(NaoProvider::link(HAND_OFFSET_X));// *
  //KinematicMatrix::transZ(NaoProvider::link(LINKS::HAND_OFFSET_Z)) *
  //KinematicMatrix::rotY( jointAngles.at(5) );

  return getRWristYaw( jointAngles ) * RHand2RWristYaw;
}



/*  +----------+
 *  | Left Leg |
 *  +----------+
 */

// LHipYawPitch
KinematicMatrix ForwardKinematics::getLHipYawPitch(const vector<float>& jointAngles)
{
  // From Torso to Hip
  KinematicMatrix LHipBase2Torso =
      KinematicMatrix::transZ(-NaoProvider::link(HIP_OFFSET_Z)) *
      KinematicMatrix::transY(NaoProvider::link(HIP_OFFSET_Y));

  // From Hip to rotated Hip
  KinematicMatrix LHipYawPitch2LHipBase =
      KinematicMatrix::rotX(-45.0f * TO_RAD) *
      KinematicMatrix::rotY(jointAngles[JOINTS_L_LEG::L_HIP_YAW_PITCH]);

  // From rotated Hip to Torso
  return LHipBase2Torso * LHipYawPitch2LHipBase;
}

// LHipRoll
KinematicMatrix ForwardKinematics::getLHipRoll(const vector<float>& jointAngles)
{
  // From LHipRoll to LHipYawPitch
  KinematicMatrix LHipRoll2LHipYawPitch =
      KinematicMatrix::rotX(45.0f * TO_RAD + jointAngles[JOINTS_L_LEG::L_HIP_ROLL]);

  // From LHipRoll to Torso
  return getLHipYawPitch( jointAngles ) * LHipRoll2LHipYawPitch;
}


// LHipPitch
KinematicMatrix ForwardKinematics::getLHipPitch(const vector<float>& jointAngles)
{
  // From LHipPitch to LHipRoll
  KinematicMatrix LHipPitch2LHipRoll =
      KinematicMatrix::rotY( jointAngles[JOINTS_L_LEG::L_HIP_PITCH] );

  // From LHipPitch to Torso
  return getLHipRoll( jointAngles ) * LHipPitch2LHipRoll;
}

// LKneePitch
KinematicMatrix ForwardKinematics::getLKneePitch(const vector<float>& jointAngles)
{
  // From LKneePitch to LHipPitch
  KinematicMatrix LKneePitch2LHipPitch =
      KinematicMatrix::transZ(-NaoProvider::link(THIGH_LENGTH)) *
      KinematicMatrix::rotY( jointAngles[JOINTS_L_LEG::L_KNEE_PITCH] );

  // From LKneePitch to Torso
  return getLHipPitch( jointAngles ) * LKneePitch2LHipPitch;
}

// LAnklePitch
KinematicMatrix ForwardKinematics::getLAnklePitch(const vector<float>& jointAngles)
{
  // From LAnklePitch to LKneePitch
  KinematicMatrix LAnklePitch2LKneePitch =
      KinematicMatrix::transZ(-NaoProvider::link(TIBIA_LENGTH)) *
      KinematicMatrix::rotY( jointAngles[JOINTS_L_LEG::L_ANKLE_PITCH] );

  // From LAnklePitch to Torso
  return getLKneePitch( jointAngles ) * LAnklePitch2LKneePitch;
}

// LAnklePitch
KinematicMatrix ForwardKinematics::getLAnkleRoll(const vector<float>& jointAngles)
{
  // From LAnkleRoll to LAnklePitch
  KinematicMatrix LAnkleRoll2LAnklePitch =
      KinematicMatrix::rotX( jointAngles[JOINTS_L_LEG::L_ANKLE_ROLL] );

  // From LAnkleRoll to Torso
  return getLAnklePitch( jointAngles ) * LAnkleRoll2LAnklePitch;
}

// LFoot
KinematicMatrix ForwardKinematics::getLFoot(const vector<float>& jointAngles)
{
  // From LFoot to LAnkleRoll
  KinematicMatrix LFoot2LAnkleRoll =
      KinematicMatrix::transZ(-NaoProvider::link(FOOT_HEIGHT));

  // From LFoot to Torso
  return getLAnkleRoll( jointAngles ) * LFoot2LAnkleRoll;
}


/*  +-----------+
 *  | Right Leg |
 *  +-----------+
 */


// RHipYawPitch
KinematicMatrix ForwardKinematics::getRHipYawPitch(const vector<float>& jointAngles)
{
  // From Torso to Hip
  KinematicMatrix RHipBase2Torso =
      KinematicMatrix::transZ(-NaoProvider::link(HIP_OFFSET_Z)) *
      KinematicMatrix::transY(-NaoProvider::link(HIP_OFFSET_Y));

  // From Hip to rotated Hip
  KinematicMatrix RHipYawPitch2LHipBase =
      KinematicMatrix::rotX(-135 * TO_RAD) *
      KinematicMatrix::rotY(-jointAngles[JOINTS_R_LEG::R_HIP_YAW_PITCH]);

  // From rotated Hip to Torso
  return RHipBase2Torso * RHipYawPitch2LHipBase;
}


// RHipRoll
KinematicMatrix ForwardKinematics::getRHipRoll(const vector<float>& jointAngles)
{
  // From LHipRoll to LHipYawPitch
  KinematicMatrix RHipRoll2RHipYawPitch =
      KinematicMatrix::rotX(135 * TO_RAD + jointAngles[JOINTS_R_LEG::R_HIP_ROLL]);

  // From LHipRoll to Torso
  return getRHipYawPitch( jointAngles ) * RHipRoll2RHipYawPitch;
}

// RHipPitch
KinematicMatrix ForwardKinematics::getRHipPitch(const vector<float>& jointAngles)
{
  // From RHipPitch to RHipRoll
  KinematicMatrix RHipPitch2RHipRoll =
      KinematicMatrix::rotY( jointAngles[JOINTS_R_LEG::R_HIP_PITCH] );

  // From LHipPitch to Torso
  return getRHipRoll( jointAngles ) * RHipPitch2RHipRoll;
}

// RKneePitch
KinematicMatrix ForwardKinematics::getRKneePitch(const vector<float>& jointAngles)
{
  // From RKneePitch to RHipPitch
  KinematicMatrix RKneePitch2RHipPitch =
      KinematicMatrix::transZ(-NaoProvider::link(THIGH_LENGTH)) *
      KinematicMatrix::rotY( jointAngles[JOINTS_R_LEG::R_KNEE_PITCH] );

  // From RKneePitch to Torso
  return getRHipPitch( jointAngles ) * RKneePitch2RHipPitch;
}

// RAnklePitch
KinematicMatrix ForwardKinematics::getRAnklePitch(const vector<float>& jointAngles)
{
  // From RAnklePitch to RKneePitch
  KinematicMatrix RAnklePitch2RKneePitch =
      KinematicMatrix::transZ(-NaoProvider::link(TIBIA_LENGTH)) *
      KinematicMatrix::rotY( jointAngles[JOINTS_R_LEG::R_ANKLE_PITCH] );

  // From RAnklePitch to Torso
  return getRKneePitch( jointAngles ) * RAnklePitch2RKneePitch;
}

// RAnklePitch
KinematicMatrix ForwardKinematics::getRAnkleRoll(const vector<float>& jointAngles)
{
  // From RAnkleRoll to RAnklePitch
  KinematicMatrix RAnkleRoll2RAnklePitch =
      KinematicMatrix::rotX( jointAngles[JOINTS_R_LEG::R_ANKLE_ROLL] );

  // From RAnkleRoll to Torso
  return getRAnklePitch( jointAngles ) * RAnkleRoll2RAnklePitch;
}

// RFoot
KinematicMatrix ForwardKinematics::getRFoot(const vector<float>& jointAngles)
{
  // From RFoot to RAnkleRoll
  KinematicMatrix RFoot2RAnkleRoll =
      KinematicMatrix::transZ(-NaoProvider::link(FOOT_HEIGHT));

  // From LFoot to Torso
  return getRAnkleRoll( jointAngles ) * RFoot2RAnkleRoll;
}

vector<KinematicMatrix> ForwardKinematics::getHead(const vector<float>& jointAngles)
{
  KinematicMatrix HeadYaw2Torso =
      KinematicMatrix::transZ(NaoProvider::link(NECK_OFFSET_Z)) *
      KinematicMatrix::rotZ(jointAngles[JOINTS_HEAD::HEAD_YAW]);

  KinematicMatrix HeadPitch2HeadYaw =
      KinematicMatrix::rotY(jointAngles[JOINTS_HEAD::HEAD_PITCH]);

  KinematicMatrix HeadPitch2Torso = HeadYaw2Torso * HeadPitch2HeadYaw;

  vector<KinematicMatrix> out;
  out.push_back(HeadYaw2Torso);
  out.push_back(HeadPitch2Torso);

  return out;
}

vector<KinematicMatrix> ForwardKinematics::getLArm(const vector<float>& jointAngles)
{
  KinematicMatrix LShoulderBase2Torso =
      KinematicMatrix::transZ(NaoProvider::link(SHOULDER_OFFSET_Z)) *
      KinematicMatrix::transY(NaoProvider::link(SHOULDER_OFFSET_Y));

  KinematicMatrix LShoulderPitch2LShoulderBase =
      KinematicMatrix::rotY(jointAngles[JOINTS_L_ARM::L_SHOULDER_PITCH]);

  // Left Shoulder Pitch
  KinematicMatrix LShoulderPitch2Torso = LShoulderBase2Torso * LShoulderPitch2LShoulderBase;

  KinematicMatrix LShoulderRoll2LShoulderPitch =
      KinematicMatrix::rotZ(jointAngles[JOINTS_L_ARM::L_SHOULDER_ROLL]);

  // Left Shoulder Roll
  KinematicMatrix LShoulderRoll2Torso = LShoulderPitch2Torso * LShoulderRoll2LShoulderPitch;

  KinematicMatrix LElbowYaw2LShoulderRoll =
      KinematicMatrix::transX(NaoProvider::link(UPPER_ARM_LENGTH)) *
      KinematicMatrix::transY(NaoProvider::link(ELBOW_OFFSET_Y)) *
      KinematicMatrix::rotX( jointAngles[JOINTS_L_ARM::L_ELBOW_YAW] );

  // Left Elbow Yaw
  KinematicMatrix LElbowYaw2Torso = LShoulderRoll2Torso * LElbowYaw2LShoulderRoll;


  KinematicMatrix LElbowRoll2LElbowYaw =
      KinematicMatrix::rotZ( jointAngles[JOINTS_L_ARM::L_ELBOW_ROLL] );

  // Left Elbow Roll
  KinematicMatrix LElbowRoll2Torso = LElbowYaw2Torso * LElbowRoll2LElbowYaw;

  KinematicMatrix LWristYaw2LElbowRoll =
      KinematicMatrix::transX(NaoProvider::link(LOWER_ARM_LENGTH)) *
      KinematicMatrix::rotX( jointAngles[JOINTS_L_ARM::L_WRIST_YAW] );

  // Left Wrist Yaw
  KinematicMatrix LWrist2Torso = LElbowRoll2Torso * LWristYaw2LElbowRoll;

  KinematicMatrix LHand2LWristYaw =
      KinematicMatrix::transX(NaoProvider::link(HAND_OFFSET_X)); //*
  //KinematicMatrix::transZ(NaoProvider::link(LINKS::HAND_OFFSET_Z)) *
  //KinematicMatrix::rotY( jointAngles.at(5) );

  KinematicMatrix LHand2Torso = LWrist2Torso * LHand2LWristYaw;

  vector<KinematicMatrix> out(6);
  out[0] = LShoulderPitch2Torso;
  out[1] = LShoulderRoll2Torso;
  out[2] = LElbowYaw2Torso;
  out[3] = LElbowRoll2Torso;
  out[4] = LWrist2Torso;
  out[5] = LHand2Torso;

  return out;



}
vector<KinematicMatrix> ForwardKinematics::getRArm(const vector<float>& jointAngles)
{
  KinematicMatrix RShoulderBase2Torso =
      KinematicMatrix::transZ(NaoProvider::link(SHOULDER_OFFSET_Z)) *
      KinematicMatrix::transY(-NaoProvider::link(SHOULDER_OFFSET_Y));

  KinematicMatrix RShoulderPitch2RShoulderBase =
      KinematicMatrix::rotY(jointAngles[JOINTS_R_ARM::R_SHOULDER_PITCH]);

  // Right Shoulder Pitch
  KinematicMatrix RShoulderPitch2Torso = RShoulderBase2Torso * RShoulderPitch2RShoulderBase;

  KinematicMatrix RShoulderRoll2RShoulderPitch =
      KinematicMatrix::rotZ(jointAngles[JOINTS_R_ARM::R_SHOULDER_ROLL]);

  // Right Shoulder Roll
  KinematicMatrix RShoulderRoll2Torso = RShoulderPitch2Torso * RShoulderRoll2RShoulderPitch;

  KinematicMatrix RElbowYaw2RShoulderRoll =
      KinematicMatrix::transX(NaoProvider::link(UPPER_ARM_LENGTH)) *
      KinematicMatrix::transY(-NaoProvider::link(ELBOW_OFFSET_Y)) *
      KinematicMatrix::rotX( jointAngles[JOINTS_R_ARM::R_ELBOW_YAW] );

  // Right Elbow Yaw
  KinematicMatrix RElbowYaw2Torso = RShoulderRoll2Torso * RElbowYaw2RShoulderRoll;

  KinematicMatrix RElbowRoll2RElbowYaw =
      KinematicMatrix::rotZ( jointAngles[JOINTS_R_ARM::R_ELBOW_ROLL] );

  // Right Elbow Roll
  KinematicMatrix RElbowRoll2Torso = RElbowYaw2Torso * RElbowRoll2RElbowYaw;

  KinematicMatrix RWristYaw2RElbowRoll =
      KinematicMatrix::transX(NaoProvider::link(LOWER_ARM_LENGTH)) *
      KinematicMatrix::rotX( jointAngles[JOINTS_R_ARM::R_WRIST_YAW] );

  // Right Wrist Yaw
  KinematicMatrix RWristYaw2Torso = RElbowRoll2Torso * RWristYaw2RElbowRoll;

  KinematicMatrix RHand2RWristYaw =
      KinematicMatrix::transX(NaoProvider::link(HAND_OFFSET_X));// *
  //KinematicMatrix::transZ(NaoProvider::link(LINKS::HAND_OFFSET_Z)) *
  //KinematicMatrix::rotY( jointAngles.at(5) );

  KinematicMatrix RHand2Torso = RWristYaw2Torso * RHand2RWristYaw;

  vector<KinematicMatrix> out(6);
  out[0] = RShoulderPitch2Torso;
  out[1] = RShoulderRoll2Torso;
  out[2] = RElbowYaw2Torso;
  out[3] = RElbowRoll2Torso;
  out[4] = RWristYaw2Torso;
  out[5] = RHand2Torso;

  return out;
}
vector<KinematicMatrix> ForwardKinematics::getLLeg(const vector<float>& jointAngles)
{
  // From Torso to Hip
  KinematicMatrix LHipBase2Torso =
      KinematicMatrix::transZ(-NaoProvider::link(HIP_OFFSET_Z)) *
      KinematicMatrix::transY(NaoProvider::link(HIP_OFFSET_Y));

  // From Hip to rotated Hip
  KinematicMatrix LHipYawPitch2LHipBase =
      KinematicMatrix::rotX(-45.0f * TO_RAD) *
      KinematicMatrix::rotY(jointAngles[JOINTS_L_LEG::L_HIP_YAW_PITCH]);

  // From rotated Hip to Torso
  KinematicMatrix LHipYawPitch2Torso =  LHipBase2Torso * LHipYawPitch2LHipBase;

  // From LHipRoll to LHipYawPitch
  KinematicMatrix LHipRoll2LHipYawPitch =
      KinematicMatrix::rotX(45.0f * TO_RAD + jointAngles[JOINTS_L_LEG::L_HIP_ROLL]);

  // From LHipRoll to Torso
  KinematicMatrix LHipRoll2Torso =  LHipYawPitch2Torso * LHipRoll2LHipYawPitch;

  // From LHipPitch to LHipRoll
  KinematicMatrix LHipPitch2LHipRoll =
      KinematicMatrix::rotY( jointAngles[JOINTS_L_LEG::L_HIP_PITCH] );

  // From LHipPitch to Torso
  KinematicMatrix LHipPitch2Torso = LHipRoll2Torso * LHipPitch2LHipRoll;

  // From LKneePitch to LHipPitch
  KinematicMatrix LKneePitch2LHipPitch =
      KinematicMatrix::transZ(-NaoProvider::link(THIGH_LENGTH)) *
      KinematicMatrix::rotY( jointAngles[JOINTS_L_LEG::L_KNEE_PITCH] );

  // From LKneePitch to Torso
  KinematicMatrix LKneePitch2Torso = LHipPitch2Torso * LKneePitch2LHipPitch;

  // From LAnklePitch to LKneePitch
  KinematicMatrix LAnklePitch2LKneePitch =
      KinematicMatrix::transZ(-NaoProvider::link(TIBIA_LENGTH)) *
      KinematicMatrix::rotY( jointAngles[JOINTS_L_LEG::L_ANKLE_PITCH] );

  // From LAnklePitch to Torso
  KinematicMatrix LAnklePitch2Torso = LKneePitch2Torso * LAnklePitch2LKneePitch;

  // From LAnkleRoll to LAnklePitch
  KinematicMatrix LAnkleRoll2LAnklePitch =
      KinematicMatrix::rotX( jointAngles[JOINTS_L_LEG::L_ANKLE_ROLL] );

  // From LAnkleRoll to Torso
  KinematicMatrix LAnkleRoll2Torso = LAnklePitch2Torso * LAnkleRoll2LAnklePitch;

  // From LFoot to LAnkleRoll
  KinematicMatrix LFoot2LAnkleRoll =
      KinematicMatrix::transZ(-NaoProvider::link(FOOT_HEIGHT));

  // From LFoot to Torso
  KinematicMatrix LFoot2Torso = LAnkleRoll2Torso * LFoot2LAnkleRoll;

  vector<KinematicMatrix> out(7);
  out[0] = LHipYawPitch2Torso;
  out[1] = LHipRoll2Torso;
  out[2] = LHipPitch2Torso;
  out[3] = LKneePitch2Torso;
  out[4] = LAnklePitch2Torso;
  out[5] = LAnkleRoll2Torso;
  out[6] = LFoot2Torso;

  return out;

}
vector<KinematicMatrix> ForwardKinematics::getRLeg(const vector<float>& jointAngles)
{
  // From Torso to Hip
  KinematicMatrix RHipBase2Torso =
      KinematicMatrix::transZ(-NaoProvider::link(HIP_OFFSET_Z)) *
      KinematicMatrix::transY(-NaoProvider::link(HIP_OFFSET_Y));

  // From Hip to rotated Hip
  KinematicMatrix RHipYawPitch2LHipBase =
      KinematicMatrix::rotX(-135 * TO_RAD) *
      KinematicMatrix::rotY(-jointAngles[JOINTS_R_LEG::R_HIP_YAW_PITCH]);

  // From rotated Hip to Torso
  KinematicMatrix RHipYawPitch2Torso = RHipBase2Torso * RHipYawPitch2LHipBase;

  // From LHipRoll to LHipYawPitch
  KinematicMatrix RHipRoll2RHipYawPitch =
      KinematicMatrix::rotX(135 * TO_RAD + jointAngles[JOINTS_R_LEG::R_HIP_ROLL]);

  // From LHipRoll to Torso
  KinematicMatrix RHipRoll2Torso = RHipYawPitch2Torso * RHipRoll2RHipYawPitch;

  // From RHipPitch to RHipRoll
  KinematicMatrix RHipPitch2RHipRoll =
      KinematicMatrix::rotY( jointAngles[JOINTS_R_LEG::R_HIP_PITCH] );

  // From LHipPitch to Torso
  KinematicMatrix RHipPitch2Torso = RHipRoll2Torso * RHipPitch2RHipRoll;

  // From RKneePitch to RHipPitch
  KinematicMatrix RKneePitch2RHipPitch =
      KinematicMatrix::transZ(-NaoProvider::link(THIGH_LENGTH)) *
      KinematicMatrix::rotY( jointAngles[JOINTS_R_LEG::R_KNEE_PITCH] );

  // From RKneePitch to Torso
  KinematicMatrix RKneePitch2Torso = RHipPitch2Torso * RKneePitch2RHipPitch;

  // From RAnklePitch to RKneePitch
  KinematicMatrix RAnklePitch2RKneePitch =
      KinematicMatrix::transZ(-NaoProvider::link(TIBIA_LENGTH)) *
      KinematicMatrix::rotY( jointAngles[JOINTS_R_LEG::R_ANKLE_PITCH] );

  // From RAnklePitch to Torso
  KinematicMatrix RAnklePitch2Torso = RKneePitch2Torso * RAnklePitch2RKneePitch;

  // From RAnkleRoll to RAnklePitch
  KinematicMatrix RAnkleRoll2RAnklePitch =
      KinematicMatrix::rotX( jointAngles[JOINTS_R_LEG::R_ANKLE_ROLL] );

  // From RAnkleRoll to Torso
  KinematicMatrix RAnkleRoll2Torso = RAnklePitch2Torso * RAnkleRoll2RAnklePitch;

  // From RFoot to RAnkleRoll
  KinematicMatrix RFoot2RAnkleRoll =
      KinematicMatrix::transZ(-NaoProvider::link(FOOT_HEIGHT));

  // From LFoot to Torso
  KinematicMatrix RFoot2Torso = RAnkleRoll2Torso * RFoot2RAnkleRoll;

  vector<KinematicMatrix> out(7);

  out[0] = RHipYawPitch2Torso;
  out[1] = RHipRoll2Torso;
  out[2] = RHipPitch2Torso;
  out[3] = RKneePitch2Torso;
  out[4] = RAnklePitch2Torso;
  out[5] = RAnkleRoll2Torso;
  out[6] = RFoot2Torso;

  return out;
}

vector<KinematicMatrix> ForwardKinematics::getBody(const vector<float>& jointAngles, const Vector3f& angle)
{
  vector<float> headAngles(JOINTS_HEAD::HEAD_MAX);
  vector<float> lLegAngles(JOINTS_L_LEG::L_LEG_MAX);
  vector<float> rLegAngles(JOINTS_R_LEG::R_LEG_MAX);
  vector<float> lArmAngles(JOINTS_L_ARM::L_ARM_MAX);
  vector<float> rArmAngles(JOINTS_R_ARM::R_ARM_MAX);

  for (int i = 0; i< JOINTS_HEAD::HEAD_MAX; i++)
    headAngles[i] = (jointAngles[JOINTS::HEAD_YAW+i]);

  for (int i = 0; i< JOINTS_L_LEG::L_LEG_MAX;i++)
  {
    lLegAngles[i] = (jointAngles[JOINTS::L_HIP_YAW_PITCH+i]);
    rLegAngles[i] = (jointAngles[JOINTS::R_HIP_YAW_PITCH+i]);
    lArmAngles[i] = (jointAngles[JOINTS::L_SHOULDER_PITCH+i]);
    rArmAngles[i] = (jointAngles[JOINTS::R_SHOULDER_PITCH+i]);
  }

  vector<KinematicMatrix> out = vector<KinematicMatrix>(JOINTS::JOINTS_ADD_MAX,KinematicMatrix());

  vector<KinematicMatrix> headKin =getHead(headAngles);
  vector<KinematicMatrix> lArmKin = getLArm(lArmAngles);
  vector<KinematicMatrix> lLegKin = getLLeg(lLegAngles);
  vector<KinematicMatrix> rLegKin = getRLeg(rLegAngles);
  vector<KinematicMatrix> rArmKin = getRArm(rArmAngles);

  for (int i = 0; i< JOINTS_HEAD::HEAD_MAX; i++)
    out[JOINTS::HEAD_YAW+i] = headKin[i];

  for (int i = 0; i < JOINTS_L_LEG::L_LEG_MAX; i++)
  {
    out[JOINTS::L_SHOULDER_PITCH + i] = lArmKin[i];
    out[JOINTS::L_HIP_YAW_PITCH + i] = lLegKin[i];
    out[JOINTS::R_SHOULDER_PITCH + i] = rArmKin[i];
    out[JOINTS::R_HIP_YAW_PITCH + i] = rLegKin[i];
  }

  out[JOINTS::L_FOOT] = lLegKin.back();
  out[JOINTS::R_FOOT] = rLegKin.back();

  /// calculate torso to ground with imu
  /// Is this used anywhere? I think the calculations are not useful, however I did not change the effect. ~Arne 20.7.2015
  const KinematicMatrix& foot2torso = (out[JOINTS::L_FOOT].posV.z() < out[JOINTS::R_FOOT].posV.z()) ? out[JOINTS::L_FOOT] : out[JOINTS::R_FOOT];

  KinematicMatrix torso2groundImu(KinematicMatrix::rotY(angle.y()) * KinematicMatrix::rotX(angle.x()) * KinematicMatrix(-foot2torso.posV));
  torso2groundImu.posV.x() = 0;
  torso2groundImu.posV.y() = 0;

  out[JOINTS::TORSO2GROUND_IMU] = torso2groundImu;

  /// torso2ground
  auto foot2torsoRotM = foot2torso.rotM.toRotationMatrix();
  KinematicMatrix rotation(KinematicMatrix::rotY(-asin(foot2torsoRotM(0, 2))) * KinematicMatrix::rotX(asin(foot2torsoRotM(1, 2))));
  KinematicMatrix torso2ground(rotation * KinematicMatrix(-foot2torso.posV));
  torso2ground.posV.x() = 0;
  torso2ground.posV.y() = 0;

  out[JOINTS::TORSO2GROUND] = torso2ground;

  return out;
}
