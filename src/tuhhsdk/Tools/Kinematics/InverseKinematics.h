#ifndef __InverseKinematics_h__
#define __InverseKinematics_h__


#include "KinematicMatrix.h"
#include <vector>
#include <limits>



/// Implementation of Inverse Kinematics
/**
 * This class implements the inverse kinematics for the Nao robot.
 * It calculates the joints angles for a specified position of an endeffector.
 * All positions and orientations are relative to the torso space
 * @author <a href="mailto:stefan.kaufmann@tu-harburg.de">Stefan Kaufmann</a>
 */
class InverseKinematics
{
public:
	/** default constructor */
    InverseKinematics(){}

	/**
	 * Calculation of the angles for the left leg for a specified position and rotation of the left foot
	 * @param desired a KinematicMatrix containing the rotation and position of the foot relative to the torso
     * @return A vector containing the angles for the left leg. The order of the angles is
     * defined by JOINTS_L_LEG
	 */
	static std::vector<float> getLLegAngles(const KinematicMatrix& desired);

	/**
	 * Calculation of the angles for the left leg for a specified position and rotation of the left foot
	 * @param desired a KinematicMatrix containing the rotation and position of the foot relative to the torso
     * @return A vector containing the angles for the right leg. The order of the angles is
     * defined by JOINTS_R_LEG
	 */
	static std::vector<float> getRLegAngles(const KinematicMatrix& desired);

	/**
	 * Calculation of the left leg angles with a given HipYawPitch joint value
	 * @param desired The desired foot position and orientation
	 * @param a_HipYawPitch The desired HipYawPitch angle
     * @return A vector containing the angles for the left leg. The order of the angles is
     * defined by JOINTS_L_LEG
	 */
	static std::vector<float> getFixedLLegAngles(const KinematicMatrix& desired, const float& a_HipYawPitch);

	/**
	 * Calculation of the right leg angles with a given HipYawPitch joint value
	 * @param desired The desired foot position and orientation
	 * @param a_HipYawPitch The desired HipYawPitch angle
     * @return A vector containing the angles for the right leg. The order of the angles is
     * defined by JOINTS_R_LEG
	 */
	static std::vector<float> getFixedRLegAngles(const KinematicMatrix& desired, const float& a_HipYawPitch);

	/**
	 * Calculation of the angles for the left arm
	 * @param desired A KinematicMatrix containing the desired Position and Orientation of the Left Hand
     * @param handOpening The Value for the joint which opens the hand (0 = closed, 1 = opened)
     * @return A vector containing the angles for the left arm. The order of the angles is
     * defined by JOINTS_L_ARM
	 */
    static std::vector<float> getLArmAngles(const KinematicMatrix& desired, const float& handOpening);

	/**
	 * Calculation of the angles for the left arm
	 * @param desired A KinematicMatrix containing the desired Position and Orientation of the Left Hand
     * @param handOpening The Value for the joint which opens the hand (0 = closed, 1 = opened)
     * @return A vector containing the angles for the right arm. The order of the angles is
     * defined by JOINTS_R_ARM
	 */
    static std::vector<float> getRArmAngles(const KinematicMatrix& desired, const float& handOpening);

private:

	/**
	* calculation of Pitch limitation curve
	* @param y the y-value for which the limit shall be calculated
	* @param k a constant depending on max ShoulderPitch angle
	* @return the x-limit of the shoulder Pitch
	*/
	static inline float getPitchlimit(const float& y, const float& k);



};
#endif
