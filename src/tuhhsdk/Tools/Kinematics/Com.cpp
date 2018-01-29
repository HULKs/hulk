/*
 * @author <a href="mailto:stefan.kaufmann@tu-harburg.de">Stefan Kaufmann</a>
 */

#include "Com.h"
#include <Modules/NaoProvider.h>

#include "ForwardKinematics.h"

/* --------------*/
/*	  Left Leg	 */
/* --------------*/

using namespace ELEMENTS;
using namespace std;

// com
Vector3f Com::getComLLeg(const vector<KinematicMatrix>& legKin)
{
	// calculate the com positions relative to the torso

	// LPelvis calculate postion vector
	const KinematicMatrix& lPelvis		=	legKin[0];
  Vector3f r_LPelvis	=	lPelvis * NaoProvider::com(L_PELVIS);

	// RHip
	const KinematicMatrix& lHipRoll	=	legKin[1];
  Vector3f r_LHip		=	lHipRoll * NaoProvider::com(L_HIP);

	// RThigh
	const KinematicMatrix& lHipPitch	=	legKin[2];
  Vector3f r_LThigh		=	lHipPitch * NaoProvider::com(L_THIGH);

	// RTibia
	const KinematicMatrix& lKneePitch	=	legKin[3];
  Vector3f r_LTibia		=	lKneePitch * NaoProvider::com(L_TIBIA);

	// RAnkle
	const KinematicMatrix& lAnklePitch	=	legKin[4];
  Vector3f r_LAnkle		=	lAnklePitch * NaoProvider::com(L_ANKLE);

	// RFoot
	const KinematicMatrix& lAnkleRoll	=	legKin[5];
  Vector3f r_LFoot		=	lAnkleRoll * NaoProvider::com(L_FOOT);

	// sumproduct of mass and position vectors
	Vector3f rLegComSumProduct	=
      r_LPelvis	* NaoProvider::mass(L_PELVIS)	+
      r_LHip		* NaoProvider::mass(L_HIP)			+
      r_LThigh	* NaoProvider::mass(L_THIGH)		+
      r_LTibia	* NaoProvider::mass(L_TIBIA)		+
      r_LAnkle	* NaoProvider::mass(L_ANKLE)		+
      r_LFoot		* NaoProvider::mass(L_FOOT);

	// position vector of center of mass
	return rLegComSumProduct / getMassLLeg();
}

// mass
float Com::getMassLLeg()
{
  return	NaoProvider::mass(L_PELVIS)	+
      NaoProvider::mass(L_HIP)					+
      NaoProvider::mass(L_THIGH)				+
      NaoProvider::mass(L_TIBIA)				+
      NaoProvider::mass(L_ANKLE)				+
      NaoProvider::mass(L_FOOT);
}

/* --------------*/
/*	 Right Leg	 */
/* --------------*/

// com
Vector3f Com::getComRLeg(const vector<KinematicMatrix>& legKin)
{
	// calculate the com positions relative to the torso

	// RPelvis calculate postion vector
	const KinematicMatrix& rPelvis		=	legKin[0];
  Vector3f r_RPelvis	=	rPelvis * NaoProvider::com(R_PELVIS);
	// RHip
	const KinematicMatrix& rHipRoll	=	legKin[1];
  Vector3f r_RHip		  =	rHipRoll * NaoProvider::com(R_HIP);

	// RThigh
	const KinematicMatrix& rHipPitch	=	legKin[2];
  Vector3f r_RThigh		=	rHipPitch * NaoProvider::com(R_THIGH);

	// RTibia
	const KinematicMatrix& rKneePitch	=	legKin[3];
  Vector3f r_RTibia		=	rKneePitch * NaoProvider::com(R_TIBIA);

	// RAnkle
	const KinematicMatrix& rAnklePitch	=	legKin[4];
  Vector3f r_RAnkle		=	rAnklePitch* NaoProvider::com(R_ANKLE);

	// RFoot
	const KinematicMatrix& rAnkleRoll	=	legKin[5];
  Vector3f r_RFoot		=	rAnkleRoll * NaoProvider::com(R_FOOT);

	// sumproduct of mass and position vectors
	Vector3f rLegComSumProduct	=
      r_RPelvis	* NaoProvider::mass(R_PELVIS)	+
      r_RHip		* NaoProvider::mass(R_HIP)			+
      r_RThigh	* NaoProvider::mass(R_THIGH)		+
      r_RTibia	* NaoProvider::mass(R_TIBIA)		+
      r_RAnkle	* NaoProvider::mass(R_ANKLE)		+
      r_RFoot		* NaoProvider::mass(R_FOOT);

	// position vector of center of mass
	return rLegComSumProduct / getMassRLeg();
}

// mass
float Com::getMassRLeg()
{
  return	NaoProvider::mass(R_PELVIS)	+
      NaoProvider::mass(R_HIP)					+
      NaoProvider::mass(R_THIGH)				+
      NaoProvider::mass(R_TIBIA)				+
      NaoProvider::mass(R_ANKLE)				+
      NaoProvider::mass(R_FOOT);
}


/* --------------*/
/*	  Left Arm	 */
/* --------------*/

// com
Vector3f Com::getComLArm(const vector<KinematicMatrix>& armKin)
{
	// shoulder
	const KinematicMatrix& lShoulderPitch	=	armKin[0];
  Vector3f r_LShoulder	=	lShoulderPitch * NaoProvider::com(L_SHOULDER);

	// bicep
	const KinematicMatrix& lShoulderRoll	=	armKin[1];
  Vector3f r_LBicep			=	lShoulderRoll * NaoProvider::com(L_BICEP);

	// elbow
	const KinematicMatrix& lElbowYaw		=	armKin[2];
  Vector3f r_LElbow			=	lElbowYaw * NaoProvider::com(L_ELBOW);

	// forarm
	const KinematicMatrix& lElbowRoll		=	armKin[3];
  Vector3f r_LForeArm		=	lElbowRoll * NaoProvider::com(L_FOREARM);

	// hand
	const KinematicMatrix& lHand			=	armKin[4];
  Vector3f r_LHand			=	lHand * NaoProvider::com(L_HAND);

	// sumproduct of mass and position vectors
	Vector3f lArmComSumProduct	=
      r_LShoulder	* NaoProvider::mass(L_SHOULDER)	+
      r_LBicep	* NaoProvider::mass(L_BICEP)				+
      r_LElbow	* NaoProvider::mass(L_ELBOW)				+
      r_LForeArm	* NaoProvider::mass(L_FOREARM)		+
      r_LHand		* NaoProvider::mass(L_HAND);


	return lArmComSumProduct / getMassLArm();
}

// mass
float Com::getMassLArm()
{
  return	NaoProvider::mass(L_SHOULDER)	+
      NaoProvider::mass(L_BICEP)					+
      NaoProvider::mass(L_ELBOW)					+
      NaoProvider::mass(L_FOREARM)				+
      NaoProvider::mass(L_HAND);
}


/* --------------*/
/*	 Right Arm	 */
/* --------------*/

// com

Vector3f Com::getComRArm(const vector<KinematicMatrix>& armKin)
{
	// shoulder
	const KinematicMatrix& rShoulderPitch	=	armKin[0];
  Vector3f r_RShoulder	=	rShoulderPitch * NaoProvider::com(R_SHOULDER);

	// bicep
	const KinematicMatrix& rShoulderRoll	=	armKin[1];
  Vector3f r_RBicep			=	rShoulderRoll * NaoProvider::com(R_BICEP);

	// elbow
	const KinematicMatrix& rElbowYaw		=	armKin[2];
  Vector3f r_RElbow			=	rElbowYaw * NaoProvider::com(R_ELBOW);

	// forarm
	const KinematicMatrix& rElbowRoll		=	armKin[3];
  Vector3f r_RForeArm		=	rElbowRoll * NaoProvider::com(R_FOREARM);

	// hand
	const KinematicMatrix& rHand			=	armKin[4];
  Vector3f r_RHand			=	rHand * NaoProvider::com(R_HAND);

	// sumproduct of mass and position vectors
	Vector3f rArmComSumProduct	=
      r_RShoulder	* NaoProvider::mass(R_SHOULDER)	+
      r_RBicep	* NaoProvider::mass(R_BICEP)				+
      r_RElbow	* NaoProvider::mass(R_ELBOW)				+
      r_RForeArm	* NaoProvider::mass(R_FOREARM)		+
      r_RHand		* NaoProvider::mass(R_HAND);


	return rArmComSumProduct / getMassRArm();
}

// mass
float Com::getMassRArm()
{
  return	NaoProvider::mass(R_SHOULDER)	+
      NaoProvider::mass(R_BICEP)					+
      NaoProvider::mass(R_ELBOW)					+
      NaoProvider::mass(R_FOREARM)				+
      NaoProvider::mass(R_HAND);
}


/* --------------*/
/*	    Head     */
/* --------------*/

// com

Vector3f Com::getComHead(const vector<KinematicMatrix>& headKin)
{

	// HeadYaw
	const KinematicMatrix& headYaw				=	headKin[0];
  Vector3f r_HeadYaw			   =	headYaw * NaoProvider::com(NECK);

	// HeadPitch
	const KinematicMatrix& headPitch			=	headKin[1];
  Vector3f r_HeadPitch			 =	headPitch * NaoProvider::com(HEAD);

	Vector3f headComSumProduct =
      r_HeadYaw	* NaoProvider::mass(NECK) +
      r_HeadPitch	* NaoProvider::mass(HEAD);

	return headComSumProduct / getMassHead();
}

// mass
float Com::getMassHead()
{
  return	NaoProvider::mass(NECK) +
      NaoProvider::mass(HEAD);
}

/* --------------*/
/*	    Body     */
/* --------------*/

// mass
float Com::getMassBody()
{
	return	getMassHead()	+
			getMassLArm()	+
			getMassRArm()	+
			getMassLLeg()	+
			getMassRLeg()	+
      NaoProvider::mass(TORSO);
}

/* --------------*/
/*   individual  */
/* --------------*/

Vector3f Com::getCom(const vector<float>& jointAngles)
{
	vector<float> headAngles(2);
	vector<float> lArmAngles(6);
	vector<float> rArmAngles(6);
	vector<float> lLegAngles(6);
	vector<float> rLegAngles(6);

	headAngles[0] = jointAngles[0];
	headAngles[1] = jointAngles[1];

	// initialize joint angle vectors
	for (int i = 0; i < 6; i++)
	{
		lArmAngles[i] = jointAngles[i + JOINTS::L_SHOULDER_PITCH];
		rArmAngles[i] = jointAngles[i + JOINTS::R_SHOULDER_PITCH];
		lLegAngles[i] = jointAngles[i + JOINTS::L_HIP_YAW_PITCH];
		rLegAngles[i] = jointAngles[i + JOINTS::R_HIP_YAW_PITCH];
	}

	// get Kinematic matrices to joints
	vector<KinematicMatrix> headKin = ForwardKinematics::getHead(headAngles);
	vector<KinematicMatrix> lArmKin = ForwardKinematics::getLArm(lArmAngles);
	vector<KinematicMatrix> rArmKin = ForwardKinematics::getRArm(rArmAngles);
	vector<KinematicMatrix> lLegKin = ForwardKinematics::getLLeg(lLegAngles);
	vector<KinematicMatrix> rLegKin = ForwardKinematics::getRLeg(rLegAngles);

	Vector3f BodyComSumProduct	=
			getComHead(headKin)	*	getMassHead()	+
											getComLArm(lArmKin)	*	getMassLArm()	+
											getComRArm(rArmKin)	*	getMassRArm()	+
											getComLLeg(lLegKin)	*	getMassLLeg()	+
											getComRLeg(rLegKin)	*	getMassRLeg()	+
                      NaoProvider::com(TORSO) * NaoProvider::mass(TORSO);
	return BodyComSumProduct / getMassBody();
}

Vector3f Com::getComBody(const std::vector<KinematicMatrix>& kinematicMatrices)
{
  std::vector<KinematicMatrix> headKin(2), lArmKin(6), rArmKin(6), lLegKin(6), rLegKin(6);
  headKin[0] = kinematicMatrices[JOINTS::HEAD_YAW];
  headKin[1] = kinematicMatrices[JOINTS::HEAD_PITCH];
  for (unsigned int i = 0; i < 6; i++)
  {
    lArmKin[i] = kinematicMatrices[i + JOINTS::L_SHOULDER_PITCH];
    rArmKin[i] = kinematicMatrices[i + JOINTS::R_SHOULDER_PITCH];
    lLegKin[i] = kinematicMatrices[i + JOINTS::L_HIP_YAW_PITCH];
    rLegKin[i] = kinematicMatrices[i + JOINTS::R_HIP_YAW_PITCH];
  }

  Vector3f bodyComSumProduct =
    getComHead(headKin) * getMassHead()
  + getComLArm(lArmKin) * getMassLArm()
  + getComRArm(rArmKin) * getMassRArm()
  + getComLLeg(lLegKin) * getMassLLeg()
  + getComRLeg(rLegKin) * getMassRLeg()
  + NaoProvider::com(TORSO) * NaoProvider::mass(TORSO);
  return bodyComSumProduct / getMassBody();
}
