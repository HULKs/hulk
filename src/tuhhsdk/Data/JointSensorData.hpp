#pragma once

#include <array>
#include <vector>

#include "Definitions/keys.h"
#include "Framework/DataType.hpp"


class JointSensorData : public DataType<JointSensorData>
{
public:
  /// the angles of all joints
  std::array<float, keys::joints::JOINTS_MAX> angles;
  /// the currents of all joints
  std::array<float, keys::joints::JOINTS_MAX> currents;
  /// the temperatures of all joints
  std::array<float, keys::joints::JOINTS_MAX> temperatures;
  /// the status of all joints
  std::array<float, keys::joints::JOINTS_MAX> status;
  /**
   * @brief getHeadAngles returns a vector of all head angles for Blackboard compatibility
   * @return a vector of all head angles
   */
  std::vector<float> getHeadAngles() const
  {
    return std::vector<float>(angles.begin() + keys::joints::HEAD_YAW, angles.begin() + keys::joints::HEAD_PITCH + 1);
  }
  /**
   * @brief getLArmAngles returns a vector of all left arm angles for Blackboard compatibility
   * @return a vector of all left arm angles
   */
  std::vector<float> getLArmAngles() const
  {
    return std::vector<float>(angles.begin() + keys::joints::L_SHOULDER_PITCH, angles.begin() + keys::joints::L_HAND + 1);
  }
  /**
   * @brief getRArmAngles returns a vector of all right arm angles for Blackboard compatibility
   * @return a vector of all right arm angles
   */
  std::vector<float> getRArmAngles() const
  {
    return std::vector<float>(angles.begin() + keys::joints::R_SHOULDER_PITCH, angles.begin() + keys::joints::R_HAND + 1);
  }
  /**
   * @brief getLLegAngles returns a vector of all left leg angles for Blackboard compatibility
   * @return a vector of all left leg angles
   */
  std::vector<float> getLLegAngles() const
  {
    return std::vector<float>(angles.begin() + keys::joints::L_HIP_YAW_PITCH, angles.begin() + keys::joints::L_ANKLE_ROLL + 1);
  }
  /**
   * @brief getRLegAngles returns a vector of all right leg angles for Blackboard compatibility
   * @return a vector of all right leg angles
   */
  std::vector<float> getRLegAngles() const
  {
    return std::vector<float>(angles.begin() + keys::joints::R_HIP_YAW_PITCH, angles.begin() + keys::joints::R_ANKLE_ROLL + 1);
  }
  /**
   * @brief getBodyAngles returns a vector of all angles for Blackboard compatibility
   * @return a vector of all angles
   */
  std::vector<float> getBodyAngles() const
  {
    return std::vector<float>(angles.begin(), angles.end());
  }
  /**
   * @brief reset does nothing
   */
  void reset()
  {
  }

  virtual void toValue(Uni::Value& value) const
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["angles"] << angles;
    value["currents"] << currents;
    value["temperatures"] << temperatures;
    value["status"] << status;
  }
  virtual void fromValue(const Uni::Value& value)
  {
    value["angles"] >> angles;
    value["currents"] >> currents;
    value["temperatures"] >> temperatures;
    value["status"] >> status;
  }
};
