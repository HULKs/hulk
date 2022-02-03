#pragma once

#include "Hardware/Definitions.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Math/KinematicMatrix.hpp"
#include <array>

class RobotMetrics;

/**
 * This implements the forward kinematics of the Nao robot.
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
 */
class ForwardKinematics
{
public:
  explicit ForwardKinematics(const RobotMetrics& robotMetrics);

  /** calculates the KinematicMatrix of the HeadYaw joint
   * @param jointAngles the angles of the joints \n \n
   * Structure of jointAngles:
   * - [0] HeadYaw angle
   * - [x] ...
   * @return the KinematicMatrix of the joint relative to the torso space
   */
  KinematicMatrix getHeadYaw(const JointsHeadArray<float>& jointAngles) const;

  /** calculates the KinematicMatrix of the HeadPitch joint
   * @param jointAngles the angles of the joints \n \n
   * Structure of jointAngles:
   *  - [0] HeadYaw angle
   *  - [1] HeadPitch angle
   *  - [x] ...
   * @return the KinematicMatrix of the joint relative to the torso space
   */
  KinematicMatrix getHeadPitch(const JointsHeadArray<float>& jointAngles) const;


  /** calculates the KinematicMatrix of the LShoulderPitch joint
   * @param jointAngles the angles of the joints \n \n
   * Structure of jointAngles:
   * - [0] LShoulderPitch angle
   * - [x] ...
   * @return the KinematicMatrix of the joint relative to the torso space
   */
  KinematicMatrix getLShoulderPitch(const JointsArmArray<float>& jointAngles) const;

  /** calculates the KinematicMatrix of the LShoulderRoll joint
   * @param jointAngles the angles of the joints \n \n
   * Structure of jointAngles:
   * - [0] LShoulderPitch angle
   * - [1] LShoulderRoll angle
   * - [x] ...
   * @return the KinematicMatrix of the joint relative to the torso space
   */
  KinematicMatrix getLShoulderRoll(const JointsArmArray<float>& jointAngles) const;

  /** calculates the KinematicMatrix of the LElbowYaw joint
   * @param jointAngles the angles of the joints \n \n
   * Structure of jointAngles:
   * - [0] LShoulderPitch angle
   * - [1] LShoulderRoll angle
   * - [2] LElbowYaw angle
   * - [x] ...
   * @return the KinematicMatrix of the joint relative to the torso space
   */
  KinematicMatrix getLElbowYaw(const JointsArmArray<float>& jointAngles) const;

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
  KinematicMatrix getLElbowRoll(const JointsArmArray<float>& jointAngles) const;

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
  KinematicMatrix getLWristYaw(const JointsArmArray<float>& jointAngles) const;

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
  KinematicMatrix getLHand(const JointsArmArray<float>& jointAngles) const;

  /** calculates the KinematicMatrix of the RShoulderPitch joint
   * @param jointAngles the angles of the joints \n \n
   * Structure of jointAngles:
   * - [0] RShoulderPitch angle
   * - [x] ...
   * @return the KinematicMatrix of the joint relative to the torso space
   */
  KinematicMatrix getRShoulderPitch(const JointsArmArray<float>& jointAngles) const;

  /** calculates the KinematicMatrix of the RShoulderRoll joint
   * @param jointAngles the angles of the joints \n \n
   * Structure of jointAngles:
   * - [0] RShoulderPitch angle
   * - [1] RShoulderRoll angle
   * - [x] ...
   * @return the KinematicMatrix of the joint relative to the torso space
   */
  KinematicMatrix getRShoulderRoll(const JointsArmArray<float>& jointAngles) const;

  /** calculates the KinematicMatrix of the RElbowYaw joint
   * @param jointAngles the angles of the joints \n \n
   * Structure of jointAngles:
   * - [0] RShoulderPitch angle
   * - [1] RShoulderRoll angle
   * - [2] RElbowYaw angle
   * - [x] ...
   * @return the KinematicMatrix of the joint relative to the torso space
   */
  KinematicMatrix getRElbowYaw(const JointsArmArray<float>& jointAngles) const;

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
  KinematicMatrix getRElbowRoll(const JointsArmArray<float>& jointAngles) const;

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
  KinematicMatrix getRWristYaw(const JointsArmArray<float>& jointAngles) const;

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
  KinematicMatrix getRHand(const JointsArmArray<float>& jointAngles) const;


  /** calculates the KinematicMatrix of the LHipYawPitch joint
   * @param jointAngles the angles of the joints \n \n
   * Structure of jointAngles:
   * - [0] LHipYawPitch angle
   * - [x] ...
   * @return the KinematicMatrix of the joint relative to the torso space
   */
  KinematicMatrix getLHipYawPitch(const JointsLegArray<float>& jointAngles) const;

  /** calculates the KinematicMatrix of the LHipRoll joint
   * @param jointAngles the angles of the joints \n \n
   * Structure of jointAngles:
   * - [0] LHipYawPitch angle
   * - [1] LHipRoll angle
   * - [x] ...
   * @return the KinematicMatrix of the joint relative to the torso space
   */
  KinematicMatrix getLHipRoll(const JointsLegArray<float>& jointAngles) const;

  /** calculates the KinematicMatrix of the LHipPitch joint
   * @param jointAngles the angles of the joints \n \n
   * Structure of jointAngles:
   * - [0] LHipYawPitch angle
   * - [1] LHipRoll angle
   * - [2] LHipPitch angle
   * - [x] ...
   * @return the KinematicMatrix of the joint relative to the torso space
   */
  KinematicMatrix getLHipPitch(const JointsLegArray<float>& jointAngles) const;

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
  KinematicMatrix getLKneePitch(const JointsLegArray<float>& jointAngles) const;

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
  KinematicMatrix getLAnklePitch(const JointsLegArray<float>& jointAngles) const;

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
  KinematicMatrix getLAnkleRoll(const JointsLegArray<float>& jointAngles) const;

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
  KinematicMatrix getLFoot(const JointsLegArray<float>& jointAngles) const;

  /** calculates the KinematicMatrix of the RHipYawPitch joint
   * @param jointAngles the angles of the joints \n \n
   * Structure of jointAngles:
   * - [0] RHipYawPitch angle
   * - [x] ...
   * @return the KinematicMatrix of the joint relative to the torso space
   */
  KinematicMatrix getRHipYawPitch(const JointsLegArray<float>& jointAngles) const;

  /** calculates the KinematicMatrix of the RHipRoll joint
   * @param jointAngles the angles of the joints \n \n
   * Structure of jointAngles:
   * - [0] RHipYawPitch angle
   * - [1] RHipRoll angle
   * - [x] ...
   * @return the KinematicMatrix of the joint relative to the torso space
   */
  KinematicMatrix getRHipRoll(const JointsLegArray<float>& jointAngles) const;

  /** calculates the KinematicMatrix of the RHipPitch joint
   * @param jointAngles the angles of the joints \n \n
   * Structure of jointAngles:
   * - [0] RHipYawPitch angle
   * - [1] RHipRoll angle
   * - [2] RHipPitch angle
   * - [x] ...
   * @return the KinematicMatrix of the joint relative to the torso space
   */
  KinematicMatrix getRHipPitch(const JointsLegArray<float>& jointAngles) const;

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
  KinematicMatrix getRKneePitch(const JointsLegArray<float>& jointAngles) const;

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
  KinematicMatrix getRAnklePitch(const JointsLegArray<float>& jointAngles) const;

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
  KinematicMatrix getRAnkleRoll(const JointsLegArray<float>& jointAngles) const;

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
  KinematicMatrix getRFoot(const JointsLegArray<float>& jointAngles) const;

  /** calculates the KinematicMatrices of the Head joints
   * @param jointAngles the angles of the joints \n \n
   * Structure of jointAngles:
   * - [0] HeadYaw
   * - [1] HeadPitch
   * @return A vector containing the KinematicMatrices of the head joints
   * relative to the torso space
   */
  JointsHeadArray<KinematicMatrix> getHead(const JointsHeadArray<float>& jointAngles) const;

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
  JointsArmArray<KinematicMatrix> getLArm(const JointsArmArray<float>& jointAngles) const;

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
  JointsArmArray<KinematicMatrix> getRArm(const JointsArmArray<float>& jointAngles) const;

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
  JointsLegArray<KinematicMatrix> getLLeg(const JointsLegArray<float>& jointAngles) const;

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
  JointsLegArray<KinematicMatrix> getRLeg(const JointsLegArray<float>& jointAngles) const;

  /** calculates the KinematicMatrices of the whole robot
   * @param jointAngles the angles of the joints in order of JOINTS::JOINTS
   * @return A vector containing the KinematicMatrices of the whole robot relative
   * to the torso space
   */
  JointsArray<KinematicMatrix> getBody(const JointsArray<float>& jointAngles) const;

private:
  const RobotMetrics& robotMetrics_;
};
