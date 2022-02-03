#include "Hardware/Kinematics/Com.hpp"
#include "Hardware/JointUtils.hpp"
#include "Hardware/Kinematics/ForwardKinematics.hpp"
#include "Hardware/RobotMetrics.hpp"

Com::Com(const RobotMetrics& robotMetrics)
  : robotMetrics_(robotMetrics)
{
}

Vector3f Com::getComLLeg(const JointsLegArray<KinematicMatrix>& legKin) const
{
  // calculate the com positions relative to the torso

  // LPelvis calculate postion vector
  const KinematicMatrix& lPelvis = legKin[JointsLeg::HIP_YAW_PITCH];
  Vector3f rLPelvis = lPelvis * robotMetrics_.com(Elements::L_PELVIS);

  // RHip
  const KinematicMatrix& lHipRoll = legKin[JointsLeg::HIP_ROLL];
  Vector3f rLHip = lHipRoll * robotMetrics_.com(Elements::L_HIP);

  // RThigh
  const KinematicMatrix& lHipPitch = legKin[JointsLeg::HIP_PITCH];
  Vector3f rLThigh = lHipPitch * robotMetrics_.com(Elements::L_THIGH);

  // RTibia
  const KinematicMatrix& lKneePitch = legKin[JointsLeg::KNEE_PITCH];
  Vector3f rLTibia = lKneePitch * robotMetrics_.com(Elements::L_TIBIA);

  // RAnkle
  const KinematicMatrix& lAnklePitch = legKin[JointsLeg::ANKLE_PITCH];
  Vector3f rLAnkle = lAnklePitch * robotMetrics_.com(Elements::L_ANKLE);

  // RFoot
  const KinematicMatrix& lAnkleRoll = legKin[JointsLeg::ANKLE_ROLL];
  Vector3f rLFoot = lAnkleRoll * robotMetrics_.com(Elements::L_FOOT);

  // sumproduct of mass and position vectors
  Vector3f rLegComSumProduct = rLPelvis * robotMetrics_.mass(Elements::L_PELVIS) +
                               rLHip * robotMetrics_.mass(Elements::L_HIP) +
                               rLThigh * robotMetrics_.mass(Elements::L_THIGH) +
                               rLTibia * robotMetrics_.mass(Elements::L_TIBIA) +
                               rLAnkle * robotMetrics_.mass(Elements::L_ANKLE) +
                               rLFoot * robotMetrics_.mass(Elements::L_FOOT);

  // position vector of center of mass
  return rLegComSumProduct / getMassLLeg();
}

float Com::getMassLLeg() const
{
  return robotMetrics_.mass(Elements::L_PELVIS) + robotMetrics_.mass(Elements::L_HIP) +
         robotMetrics_.mass(Elements::L_THIGH) + robotMetrics_.mass(Elements::L_TIBIA) +
         robotMetrics_.mass(Elements::L_ANKLE) + robotMetrics_.mass(Elements::L_FOOT);
}

Vector3f Com::getComRLeg(const JointsLegArray<KinematicMatrix>& legKin) const
{
  // calculate the com positions relative to the torso

  // RPelvis calculate postion vector
  const KinematicMatrix& rPelvis = legKin[JointsLeg::HIP_YAW_PITCH];
  Vector3f rRPelvis = rPelvis * robotMetrics_.com(Elements::R_PELVIS);
  // RHip
  const KinematicMatrix& rHipRoll = legKin[JointsLeg::HIP_ROLL];
  Vector3f rRHip = rHipRoll * robotMetrics_.com(Elements::R_HIP);

  // RThigh
  const KinematicMatrix& rHipPitch = legKin[JointsLeg::HIP_PITCH];
  Vector3f rRThigh = rHipPitch * robotMetrics_.com(Elements::R_THIGH);

  // RTibia
  const KinematicMatrix& rKneePitch = legKin[JointsLeg::KNEE_PITCH];
  Vector3f rRTibia = rKneePitch * robotMetrics_.com(Elements::R_TIBIA);

  // RAnkle
  const KinematicMatrix& rAnklePitch = legKin[JointsLeg::ANKLE_PITCH];
  Vector3f rRAnkle = rAnklePitch * robotMetrics_.com(Elements::R_ANKLE);

  // RFoot
  const KinematicMatrix& rAnkleRoll = legKin[JointsLeg::ANKLE_ROLL];
  Vector3f rRFoot = rAnkleRoll * robotMetrics_.com(Elements::R_FOOT);

  // sumproduct of mass and position vectors
  Vector3f rLegComSumProduct = rRPelvis * robotMetrics_.mass(Elements::R_PELVIS) +
                               rRHip * robotMetrics_.mass(Elements::R_HIP) +
                               rRThigh * robotMetrics_.mass(Elements::R_THIGH) +
                               rRTibia * robotMetrics_.mass(Elements::R_TIBIA) +
                               rRAnkle * robotMetrics_.mass(Elements::R_ANKLE) +
                               rRFoot * robotMetrics_.mass(Elements::R_FOOT);

  // position vector of center of mass
  return rLegComSumProduct / getMassRLeg();
}

float Com::getMassRLeg() const
{
  return robotMetrics_.mass(Elements::R_PELVIS) + robotMetrics_.mass(Elements::R_HIP) +
         robotMetrics_.mass(Elements::R_THIGH) + robotMetrics_.mass(Elements::R_TIBIA) +
         robotMetrics_.mass(Elements::R_ANKLE) + robotMetrics_.mass(Elements::R_FOOT);
}


Vector3f Com::getComLArm(const JointsArmArray<KinematicMatrix>& armKin) const
{
  // shoulder
  const KinematicMatrix& lShoulderPitch = armKin[JointsArm::SHOULDER_PITCH];
  Vector3f rLShoulder = lShoulderPitch * robotMetrics_.com(Elements::L_SHOULDER);

  // bicep
  const KinematicMatrix& lShoulderRoll = armKin[JointsArm::SHOULDER_ROLL];
  Vector3f rLBicep = lShoulderRoll * robotMetrics_.com(Elements::L_BICEP);

  // elbow
  const KinematicMatrix& lElbowYaw = armKin[JointsArm::ELBOW_YAW];
  Vector3f rLElbow = lElbowYaw * robotMetrics_.com(Elements::L_ELBOW);

  // forarm
  const KinematicMatrix& lElbowRoll = armKin[JointsArm::ELBOW_ROLL];
  Vector3f rLForeArm = lElbowRoll * robotMetrics_.com(Elements::L_FOREARM);

  // hand
  const KinematicMatrix& lHand = armKin[JointsArm::WRIST_YAW];
  Vector3f rLHand = lHand * robotMetrics_.com(Elements::L_HAND);

  // sumproduct of mass and position vectors
  Vector3f lArmComSumProduct = rLShoulder * robotMetrics_.mass(Elements::L_SHOULDER) +
                               rLBicep * robotMetrics_.mass(Elements::L_BICEP) +
                               rLElbow * robotMetrics_.mass(Elements::L_ELBOW) +
                               rLForeArm * robotMetrics_.mass(Elements::L_FOREARM) +
                               rLHand * robotMetrics_.mass(Elements::L_HAND);


  return lArmComSumProduct / getMassLArm();
}

// mass
float Com::getMassLArm() const
{
  return robotMetrics_.mass(Elements::L_SHOULDER) + robotMetrics_.mass(Elements::L_BICEP) +
         robotMetrics_.mass(Elements::L_ELBOW) + robotMetrics_.mass(Elements::L_FOREARM) +
         robotMetrics_.mass(Elements::L_HAND);
}

Vector3f Com::getComRArm(const JointsArmArray<KinematicMatrix>& armKin) const
{
  // shoulder
  const KinematicMatrix& rShoulderPitch = armKin[JointsArm::SHOULDER_PITCH];
  Vector3f rRShoulder = rShoulderPitch * robotMetrics_.com(Elements::R_SHOULDER);

  // bicep
  const KinematicMatrix& rShoulderRoll = armKin[JointsArm::SHOULDER_ROLL];
  Vector3f rRBicep = rShoulderRoll * robotMetrics_.com(Elements::R_BICEP);

  // elbow
  const KinematicMatrix& rElbowYaw = armKin[JointsArm::ELBOW_YAW];
  Vector3f rRElbow = rElbowYaw * robotMetrics_.com(Elements::R_ELBOW);

  // forarm
  const KinematicMatrix& rElbowRoll = armKin[JointsArm::ELBOW_ROLL];
  Vector3f rRForeArm = rElbowRoll * robotMetrics_.com(Elements::R_FOREARM);

  // hand
  const KinematicMatrix& rHand = armKin[JointsArm::WRIST_YAW];
  Vector3f rRHand = rHand * robotMetrics_.com(Elements::R_HAND);

  // sumproduct of mass and position vectors
  Vector3f rArmComSumProduct = rRShoulder * robotMetrics_.mass(Elements::R_SHOULDER) +
                               rRBicep * robotMetrics_.mass(Elements::R_BICEP) +
                               rRElbow * robotMetrics_.mass(Elements::R_ELBOW) +
                               rRForeArm * robotMetrics_.mass(Elements::R_FOREARM) +
                               rRHand * robotMetrics_.mass(Elements::R_HAND);


  return rArmComSumProduct / getMassRArm();
}

float Com::getMassRArm() const
{
  return robotMetrics_.mass(Elements::R_SHOULDER) + robotMetrics_.mass(Elements::R_BICEP) +
         robotMetrics_.mass(Elements::R_ELBOW) + robotMetrics_.mass(Elements::R_FOREARM) +
         robotMetrics_.mass(Elements::R_HAND);
}

Vector3f Com::getComHead(const JointsHeadArray<KinematicMatrix>& headKin) const
{

  // HeadYaw
  const KinematicMatrix& headYaw = headKin[JointsHead::YAW];
  Vector3f rHeadYaw = headYaw * robotMetrics_.com(Elements::NECK);

  // HeadPitch
  const KinematicMatrix& headPitch = headKin[JointsHead::PITCH];
  Vector3f rHeadPitch = headPitch * robotMetrics_.com(Elements::HEAD);

  Vector3f headComSumProduct = rHeadYaw * robotMetrics_.mass(Elements::NECK) +
                               rHeadPitch * robotMetrics_.mass(Elements::HEAD);

  return headComSumProduct / getMassHead();
}

// mass
float Com::getMassHead() const
{
  return robotMetrics_.mass(Elements::NECK) + robotMetrics_.mass(Elements::HEAD);
}

float Com::getMassBody() const
{
  return getMassHead() + getMassLArm() + getMassRArm() + getMassLLeg() + getMassRLeg() +
         robotMetrics_.mass(Elements::TORSO);
}

Vector3f Com::getCom(const JointsArray<float>& jointAngles) const
{
  const auto headAngles = JointUtils::extractHead(jointAngles);
  const auto lLegAngles = JointUtils::extractLeftLeg(jointAngles);
  const auto rLegAngles = JointUtils::extractRightLeg(jointAngles);
  const auto lArmAngles = JointUtils::extractLeftArm(jointAngles);
  const auto rArmAngles = JointUtils::extractRightArm(jointAngles);

  // get Kinematic matrices to joints
  const auto headKin = robotMetrics_.forwardKinematics().getHead(headAngles);
  const auto lArmKin = robotMetrics_.forwardKinematics().getLArm(lArmAngles);
  const auto lLegKin = robotMetrics_.forwardKinematics().getLLeg(lLegAngles);
  const auto rLegKin = robotMetrics_.forwardKinematics().getRLeg(rLegAngles);
  const auto rArmKin = robotMetrics_.forwardKinematics().getRArm(rArmAngles);

  Vector3f bodyComSumProduct =
      getComHead(headKin) * getMassHead() + getComLArm(lArmKin) * getMassLArm() +
      getComRArm(rArmKin) * getMassRArm() + getComLLeg(lLegKin) * getMassLLeg() +
      getComRLeg(rLegKin) * getMassRLeg() +
      robotMetrics_.com(Elements::TORSO) * robotMetrics_.mass(Elements::TORSO);
  return bodyComSumProduct / getMassBody();
}

Vector3f Com::getComBody(const JointsArray<KinematicMatrix>& kinematicMatrices) const
{
  const auto headKin = JointUtils::extractHead(kinematicMatrices);
  const auto lArmKin = JointUtils::extractLeftArm(kinematicMatrices);
  const auto rArmKin = JointUtils::extractRightArm(kinematicMatrices);
  const auto lLegKin = JointUtils::extractLeftLeg(kinematicMatrices);
  const auto rLegKin = JointUtils::extractRightLeg(kinematicMatrices);

  const Vector3f bodyComSumProduct =
      getComHead(headKin) * getMassHead() + getComLArm(lArmKin) * getMassLArm() +
      getComRArm(rArmKin) * getMassRArm() + getComLLeg(lLegKin) * getMassLLeg() +
      getComRLeg(rLegKin) * getMassRLeg() +
      robotMetrics_.com(Elements::TORSO) * robotMetrics_.mass(Elements::TORSO);
  return bodyComSumProduct / getMassBody();
}
