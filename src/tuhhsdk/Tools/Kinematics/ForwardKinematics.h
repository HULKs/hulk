#pragma once

#include "KinematicMatrix.h"
#include "Tools/Math/Eigen.hpp"
#include <vector>

/// Implementation of Forward Kinematics.
/**
 * This class implements the forward kinematics of the Nao robot.
 * It calculates the positions of the joints from the joint angles.
 * All positions are relative to the torso space
 *
 * Some joint angles are needed as parameters to compute the positions
 * and orientations. You will have to give at least all joint angles of the previous
 * joints in a chain as well as the joint angle for the joint which you want
 * to compute. But you can also give always the joint angles of the whole chain
 * as a parameter.
 * For computing positions and orientations of all joints in a chain, there are
 * special functions available.
 *
 * @author <a href="mailto:stefan.kaufmann@tu-harburg.de">Stefan Kaufmann</a>
 */
class ForwardKinematics
{
public:

	/** default constructor */
    ForwardKinematics(){}


	/*  +----------+
	 *  |   Head   |
	 *  +----------+
	 */

	/** calculates the KinematicMatrix of the HeadYaw joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] HeadYaw angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getHeadYaw(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the HeadPitch joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 *  - [0] HeadYaw angle
	 *  - [1] HeadPitch angle
	 *  - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getHeadPitch(const std::vector<float>& jointAngles);


	/*  +----------+
	 *  | Left Arm |
	 *  +----------+
	 */

	/** calculates the KinematicMatrix of the LShoulderPitch joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] LShoulderPitch angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getLShoulderPitch(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the LShoulderRoll joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] LShoulderPitch angle
	 * - [1] LShoulderRoll angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getLShoulderRoll(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the LElbowYaw joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] LShoulderPitch angle
	 * - [1] LShoulderRoll angle
	 * - [2] LElbowYaw angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getLElbowYaw(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the LElbowRoll joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] LShoulderPitch angle
	 * - [1] LShoulderRoll angle
	 * - [2] LElbowYaw angle
	 * - [3] LElbowRoll angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getLElbowRoll(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the LWristYaw joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] LShoulderPitch angle
	 * - [1] LShoulderRoll angle
	 * - [2] LElbowYaw angle
	 * - [3] LElbowRoll angle
	 * - [4] LWristYaw angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getLWristYaw(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the LHand joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] LShoulderPitch angle
	 * - [1] LShoulderRoll angle
	 * - [2] LElbowYaw angle
	 * - [3] LElbowRoll angle
	 * - [4] LWristYaw angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getLHand(const std::vector<float>& jointAngles);

	/*  +-----------+
	 *  | Right Arm |
	 *  +-----------+
	 */

	/** calculates the KinematicMatrix of the RShoulderPitch joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] RShoulderPitch angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getRShoulderPitch(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the RShoulderRoll joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] RShoulderPitch angle
	 * - [1] RShoulderRoll angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getRShoulderRoll(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the RElbowYaw joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] RShoulderPitch angle
	 * - [1] RShoulderRoll angle
	 * - [2] RElbowYaw angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getRElbowYaw(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the RElbowRoll joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] RShoulderPitch angle
	 * - [1] RShoulderRoll angle
	 * - [2] RElbowYaw angle
	 * - [3] RElbowRoll angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getRElbowRoll(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the RWristYaw joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] RShoulderPitch angle
	 * - [1] RShoulderRoll angle
	 * - [2] RElbowYaw angle
	 * - [3] RElbowRoll angle
	 * - [4] RWristYaw angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getRWristYaw(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the RHand joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] RShoulderPitch angle
	 * - [1] RShoulderRoll angle
	 * - [2] RElbowYaw angle
	 * - [3] RElbowRoll angle
	 * - [4] RWristYaw angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getRHand(const std::vector<float>& jointAngles);




	/*  +----------+
	 *  | Left Leg |
	 *  +----------+
	 */

	/** calculates the KinematicMatrix of the LHipYawPitch joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] LHipYawPitch angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getLHipYawPitch(const std::vector<float>& jointAngles);

    /** calculates the KinematicMatrix of the LHipRoll joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] LHipYawPitch angle
	 * - [1] LHipRoll angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getLHipRoll(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the LHipPitch joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] LHipYawPitch angle
	 * - [1] LHipRoll angle
	 * - [2] LHipPitch angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getLHipPitch(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the LKneePitch joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] LHipYawPitch angle
	 * - [1] LHipRoll angle
	 * - [2] LHipPitch angle
	 * - [3] LKneePitch angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getLKneePitch(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the LAnklePitch joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] LHipYawPitch angle
	 * - [1] LHipRoll angle
	 * - [2] LHipPitch angle
	 * - [3] LKneePitch angle
	 * - [4] LAnklePitch angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getLAnklePitch(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the LAnkleRoll joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] LHipYawPitch angle
	 * - [1] LHipRoll angle
	 * - [2] LHipPitch angle
	 * - [3] LKneePitch angle
	 * - [4] LAnklePitch angle
	 * - [5] LAnkleRoll angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getLAnkleRoll(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the LFoot joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] LHipYawPitch angle
	 * - [1] LHipRoll angle
	 * - [2] LHipPitch angle
	 * - [3] LKneePitch angle
	 * - [4] LAnklePitch angle
	 * - [5] LAnkleRoll angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getLFoot(const std::vector<float>& jointAngles);

	/*  +-----------+
	 *  | Right Leg |
	 *  +-----------+
	 */

	/** calculates the KinematicMatrix of the RHipYawPitch joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] RHipYawPitch angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getRHipYawPitch(const std::vector<float>& jointAngles);

    /** calculates the KinematicMatrix of the RHipRoll joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] RHipYawPitch angle
	 * - [1] RHipRoll angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getRHipRoll(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the RHipPitch joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] RHipYawPitch angle
	 * - [1] RHipRoll angle
	 * - [2] RHipPitch angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getRHipPitch(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the RKneePitch joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] RHipYawPitch angle
	 * - [1] RHipRoll angle
	 * - [2] RHipPitch angle
	 * - [3] RKneePitch angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getRKneePitch(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the RAnklePitch joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] RHipYawPitch angle
	 * - [1] RHipRoll angle
	 * - [2] RHipPitch angle
	 * - [3] RKneePitch angle
	 * - [4] RAnklePitch angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getRAnklePitch(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the RAnkleRoll joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] RHipYawPitch angle
	 * - [1] RHipRoll angle
	 * - [2] RHipPitch angle
	 * - [3] RKneePitch angle
	 * - [4] RAnklePitch angle
	 * - [5] RAnkleRoll angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getRAnkleRoll(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrix of the RFoot joint
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] RHipYawPitch angle
	 * - [1] RHipRoll angle
	 * - [2] RHipPitch angle
	 * - [3] RKneePitch angle
	 * - [4] RAnklePitch angle
	 * - [5] RAnkleRoll angle
	 * - [x] ...
	 * @return the KinematicMatrix of the joint relative to the torso space
	 */
	static KinematicMatrix getRFoot(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrices of the Head joints
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] HeadYaw
	 * - [1] HeadPitch
	 * @return A vector containing the KinematicMatrices of the head joints
	 * relative to the torso space
	 */
	static std::vector<KinematicMatrix> getHead(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrices of the left arm joints
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] LShoulderPitch angle
	 * - [1] LShoulderRoll angle
	 * - [2] LElbowYaw angle
	 * - [3] LElbowRoll angle
	 * - [4] LWristYaw angle
	 * @return A vector containing the KinematicMatrices of the left arm joints
	 * relative to the torso space
	 */
	static std::vector<KinematicMatrix> getLArm(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrices of the right arm joints
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] RShoulderPitch angle
	 * - [1] RShoulderRoll angle
	 * - [2] RElbowYaw angle
	 * - [3] RElbowRoll angle
	 * - [4] RWristYaw angle
	 * @return A vector containing the KinematicMatrices of the right arm joints
	 * relative to the torso space
	 */
	static std::vector<KinematicMatrix> getRArm(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrices of the left leg joints
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] LHipYawPitch angle
	 * - [1] LHipRoll angle
	 * - [2] LHipPitch angle
	 * - [3] LKneePitch angle
	 * - [4] LAnklePitch angle
	 * - [5] LAnkleRoll angle
	 * @return A vector containing the KinematicMatrices of the left leg joints
	 * relative to the torso space
	 */
	static std::vector<KinematicMatrix> getLLeg(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrices of the right leg joints
	 * @param jointAngles the angles of the joints \n \n
	 * Structure of jointAngles:
	 * - [0] RHipYawPitch angle
	 * - [1] RHipRoll angle
	 * - [2] RHipPitch angle
	 * - [3] RKneePitch angle
	 * - [4] RAnklePitch angle
	 * - [5] RAnkleRoll angle
	 * @return A vector containing the KinematicMatrices of the right leg joints
	 * relative to the torso space
	 */
	static std::vector<KinematicMatrix> getRLeg(const std::vector<float>& jointAngles);

	/** calculates the KinematicMatrices of the whole robot
	 * @param jointAngles the angles of the joints in order of JOINTS::JOINTS
	 * @param angle the angles of the torso as measured/estimated by the IMU
	 * @return A vector containing the KinematicMatrices of the whole robot relative
	 * to the torso space
	 */
	static std::vector<KinematicMatrix> getBody(const std::vector<float>& jointAngles, const Vector3f& angle);
};
