#pragma once

#include "Hardware/Definitions.hpp"
#include "Tools/Math/KinematicMatrix.hpp"
#include <array>
#include <limits>

class RobotMetrics;

/**
 * This implements the inverse kinematics for the Nao robot.
 * It calculates the joints angles for a specified position of an endeffector.
 * All positions and orientations are relative to the torso space
 */
class InverseKinematics
{
public:
  explicit InverseKinematics(const RobotMetrics& robotMetrics);
  /**
   * Calculation of the angles for the left leg for a specified position and rotation of the left
   * foot
   * @param desired a KinematicMatrix containing the rotation and position of the foot relative to
   * the torso
   * @return A vector containing the angles for the left leg. The order of the angles is
   * defined by JOINTS_L_LEG
   */
  JointsLegArray<float> getLLegAngles(const KinematicMatrix& desired) const;

  /**
   * Calculation of the angles for the left leg for a specified position and rotation of the left
   * foot
   * @param desired a KinematicMatrix containing the rotation and position of the foot relative to
   * the torso
   * @return A vector containing the angles for the right leg. The order of the angles is
   * defined by JOINTS_R_LEG
   */
  JointsLegArray<float> getRLegAngles(const KinematicMatrix& desired) const;

  /**
   * Calculation of the left leg angles with a given HipYawPitch joint value
   * @param desired The desired foot position and orientation
   * @param a_HipYawPitch The desired HipYawPitch angle
   * @return A vector containing the angles for the left leg. The order of the angles is
   * defined by JOINTS_L_LEG
   */
  JointsLegArray<float> getFixedLLegAngles(const KinematicMatrix& desired,
                                           float aHipYawPitch) const;

  /**
   * Calculation of the right leg angles with a given HipYawPitch joint value
   * @param desired The desired foot position and orientation
   * @param a_HipYawPitch The desired HipYawPitch angle
   * @return A vector containing the angles for the right leg. The order of the angles is
   * defined by JOINTS_R_LEG
   */
  JointsLegArray<float> getFixedRLegAngles(const KinematicMatrix& desired,
                                           float aHipYawPitch) const;

  /**
   * Calculation of the angles for the left arm
   * @param desired A KinematicMatrix containing the desired Position and Orientation of the Left
   * Hand
   * @param handOpening The Value for the joint which opens the hand (0 = closed, 1 = opened)
   * @return A vector containing the angles for the left arm. The order of the angles is
   * defined by JOINTS_L_ARM
   */
  JointsArmArray<float> getLArmAngles(const KinematicMatrix& desired, float handOpening) const;

  /**
   * Calculation of the angles for the left arm
   * @param desired A KinematicMatrix containing the desired Position and Orientation of the Left
   * Hand
   * @param handOpening The Value for the joint which opens the hand (0 = closed, 1 = opened)
   * @return A vector containing the angles for the right arm. The order of the angles is
   * defined by JOINTS_R_ARM
   */
  JointsArmArray<float> getRArmAngles(const KinematicMatrix& desired, float handOpening) const;

private:
  /**
   * calculation of Pitch limitation curve
   * @param y the y-value for which the limit shall be calculated
   * @param k a constant depending on max ShoulderPitch angle
   * @return the x-limit of the shoulder Pitch
   */
  float getPitchLimit(float y, float k) const;

  const RobotMetrics& robotMetrics_;
};
