#pragma once

#include "Data/HeadPositionData.hpp"
#include "Framework/DataType.hpp"
#include "Hardware/Definitions.hpp"
#include <array>
#include <vector>


class JointSensorData : public DataType<JointSensorData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"JointSensorData"};
  /// the angles of all joints
  JointsArray<float> angles{};
  /// the stiffnesses of all joints
  JointsArray<float> stiffnesses{};
  /// the currents of all joints
  JointsArray<float> currents{};
  /// the temperatures of all joints
  JointsArray<float> temperatures{};
  /// the status of all joints
  JointsArray<float> status{};
  /// whether the content is valid
  bool valid = false;
  /**
   * @brief getHeadAngles returns a vector of all head angles for Blackboard compatibility
   * @return a vector of all head angles
   */
  JointsHeadArray<float> getHeadAngles() const
  {
    return {{angles[Joints::HEAD_YAW], angles[Joints::HEAD_PITCH]}};
  }
  /**
   * @brief getHeadAngles returns the current Head Position
   * @return a vector of all head angles
   */
  HeadPosition getHeadHeadPosition() const
  {
    return HeadPosition{angles[Joints::HEAD_YAW], angles[Joints::HEAD_PITCH]};
  }
  /**
   * @brief getLArmAngles returns a vector of all left arm angles for Blackboard compatibility
   * @return a vector of all left arm angles
   */
  JointsArmArray<float> getLArmAngles() const
  {
    return {{angles[Joints::L_SHOULDER_PITCH], angles[Joints::L_SHOULDER_ROLL],
             angles[Joints::L_ELBOW_YAW], angles[Joints::L_ELBOW_ROLL], angles[Joints::L_WRIST_YAW],
             angles[Joints::L_HAND]}};
  }
  /**
   * @brief getRArmAngles returns a vector of all right arm angles for Blackboard compatibility
   * @return a vector of all right arm angles
   */
  JointsArmArray<float> getRArmAngles() const
  {
    return {{angles[Joints::R_SHOULDER_PITCH], angles[Joints::R_SHOULDER_ROLL],
             angles[Joints::R_ELBOW_YAW], angles[Joints::R_ELBOW_ROLL], angles[Joints::R_WRIST_YAW],
             angles[Joints::R_HAND]}};
  }
  /**
   * @brief getLLegAngles returns a vector of all left leg angles for Blackboard compatibility
   * @return a vector of all left leg angles
   */
  JointsLegArray<float> getLLegAngles() const
  {
    return {{angles[Joints::L_HIP_YAW_PITCH], angles[Joints::L_HIP_ROLL],
             angles[Joints::L_HIP_PITCH], angles[Joints::L_KNEE_PITCH],
             angles[Joints::L_ANKLE_PITCH], angles[Joints::L_ANKLE_ROLL]}};
  }
  /**
   * @brief getRLegAngles returns a vector of all right leg angles for Blackboard compatibility
   * @return a vector of all right leg angles
   */
  JointsLegArray<float> getRLegAngles() const
  {
    return {{angles[Joints::R_HIP_YAW_PITCH], angles[Joints::R_HIP_ROLL],
             angles[Joints::R_HIP_PITCH], angles[Joints::R_KNEE_PITCH],
             angles[Joints::R_ANKLE_PITCH], angles[Joints::R_ANKLE_ROLL]}};
  }
  /**
   * @brief getBodyAngles returns a vector of all angles for Blackboard compatibility
   * @return a vector of all angles
   */
  const JointsArray<float>& getBodyAngles() const
  {
    return angles;
  }
  /**
   * @brief marks the content as invalid
   */
  void reset() override
  {
    valid = false;
  }

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["angles"] << angles;
    value["currents"] << currents;
    value["temperatures"] << temperatures;
    value["status"] << status;
    value["valid"] << valid;
  }
  void fromValue(const Uni::Value& value) override
  {
    value["angles"] >> angles;
    value["currents"] >> currents;
    value["temperatures"] >> temperatures;
    value["status"] >> status;
    value["valid"] >> valid;
  }
};
