#pragma once


#include "Tools/Math/Eigen.hpp"
#include "KinematicMatrix.h"
#include <string>
#include <vector>

/// Center of Mass calculation
/**
 * This class calculates the center of mass for a chain of joints
 * @author <a href="mailto:stefan.kaufmann@tu-harburg.de">Stefan Kaufmann</a>
 */
class Com
{
public:
	/** calculate the position of the center of mass of the left leg relative to the torso
	 * @param legKin A vector containing the kinematic information for the leg joints \n \n
	 * order of legKin
	 * - [0] LHipYawPitch
	 * - [1] LHipRoll
	 * - [2] LHipPitch
	 * - [3] LKneePitch
	 * - [4] LAnklePitch
	 * - [5] LAnkleRoll
	 * .
	 * legKin can be calculated by ForwardKinematics
	 * @return a vector containing the x,y,z position of the com
	 */
	static Vector3f getComLLeg(const std::vector<KinematicMatrix>& legKin);

	/** calculate the position of the center of mass of the right leg relative to the torso
	 * @param legKin A vector containing the kinematic information for the leg joints \n \n
	 * order of legKin
	 * - [0] RHipYawPitch
	 * - [1] RHipRoll
	 * - [2] RHipPitch
	 * - [3] RKneePitch
	 * - [4] RAnklePitch
	 * - [5] RAnkleRoll
	 * .
	 * legKin can be calculated by ForwardKinematics
	 * @return a vector containing the x,y,z position of the com
	 */
	static Vector3f getComRLeg(const std::vector<KinematicMatrix>& legKin);

	/** calculate the position of the center of mass of the left arm relative to the torso
	 * @param armKin A vector containing the kinematic information of the arm \n \n
	 * order of armKin:
	 * - [0] LShoulderPitch
	 * - [1] LShoulderRoll
	 * - [2] LElbowYaw
	 * - [3] LElbowRoll
	 * - [4] LWristYaw
	 * .
	 * armKin can be calculated by ForwardKinematics
	 * @return a vector containing the x,y,z position of the com
	 */
	static Vector3f getComLArm(const std::vector<KinematicMatrix>& armKin);

	/** calculate the position of the center of mass of the right arm relative to the torso
	 * @param armKin A vector containing the kinematic information of the arm \n \n
	 * order of armKin:
	 * - [0] RShoulderPitch
	 * - [1] RShoulderRoll
	 * - [2] RElbowYaw
	 * - [3] RElbowRoll
	 * - [4] RWristYaw
	 * .
	 * armKin can be calculated by ForwardKinematics
	 * @return a vector containing the x,y,z position of the com
	 */
	static Vector3f getComRArm(const std::vector<KinematicMatrix>& armKin);

	/** calculate the position of the center of mass of the head relative to the torso
	 * @param headKin A vector containing the KinematicMatrices for the head \n \n
	 * order of headKin
	 * - [0] HeadYaw
	 * - [1] HeadPitch
	 * .
	 * headKin can be calculated by ForwardKinematics
	 * @return a vector containing the x,y,z position of the com
	 */
	static Vector3f getComHead(const std::vector<KinematicMatrix>& headKin);

	/** calculate the total mass of the left leg
	 * @return the total mass
	 */
	static float getMassLLeg();

	/** calculate the total mass of the right leg
	 * @return the total mass
	 */
	static float getMassRLeg();

	/** calculate the total mass of the left arm
	 * @return the total mass
	 */
	static float getMassLArm();

	/** calculate the total mass of the right arm
	 * @return the total mass
	 */
	static float getMassRArm();

	/** calculate the total mass of the head
	 * @return the total mass
	 */
	static float getMassHead();

	/** calculate the total mass of the body
	 * @return the total mass
	 */
	static float getMassBody();

	/** calculate the position of the center of mass of the body relative to the torso
	 * @param jointAngles the jointAngles for the body \n \n
	 * order of jointAngles
	 * - [0] HeadYaw
	 * - [1] HeadPitch
	 * - [2] LShoulderPitch
	 * - [3] LShoulderRoll
	 * - [4] LElbowYaw
	 * - [5] LElbowRoll
	 * - [6] LWristYaw
	 * - [7] LHand
	 * - [8] LHipYawPitch
	 * - [9] LHipRoll
	 * - [10] LHipPitch
	 * - [11] LKneePitch
	 * - [12] LAnklePitch
	 * - [13] LAnkleRoll
	 * - [14] RHipYawPitch (=LHipYawPitch)
	 * - [15] RHipRoll
	 * - [16] RHipPitch
	 * - [17] RKneePitch
	 * - [18] RAnklePitch
	 * - [19] RAnkleRoll
	 * - [20] RShoulderPitch
	 * - [21] RShoulderRoll
	 * - [22] RElbowYaw
	 * - [23] RElbowRoll
	 * - [24] RWristYaw
	 * - [25] RHand
	 * @return a vector containing the x,y,z position of the com
	 */
	static Vector3f getCom(const std::vector<float>& jointAngles);

  /**
   * @brief getComBody calculates the position of the CoM
   * @param kinematicMatrices a vector of kinematic matrices for all joint poses
   * @return a vector containing the poisition of the CoM relative to the torso
   */
  static Vector3f getComBody(const std::vector<KinematicMatrix>& kinematicMatrices);


};
