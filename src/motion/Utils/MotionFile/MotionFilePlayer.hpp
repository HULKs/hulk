#pragma once

#include <vector>

#include "Data/CycleInfo.hpp"
#include "Data/JointSensorData.hpp"
#include "Tools/Time.hpp"
#include "Definitions/keys.h"

#include "MotionFile.hpp"


class MotionFilePlayer : private MotionFile
{
public:
  /**
   * @struct JointValues
   */
  struct JointValues {
    /// all the joint angles in a frame
    std::vector<float> angles = std::vector<float>(keys::joints::JOINTS_MAX);
    /// all the joint stiffnesses in a frame
    std::vector<float> stiffnesses = std::vector<float>(keys::joints::JOINTS_MAX, -1.f);
  };
  /**
   * @brief MotionFilePlayer initializes members
   * @param cycleInfo a reference to the cycle info
   * @param jointSensorData a reference to the joint sensor data
   */
  MotionFilePlayer(const CycleInfo& cycleInfo, const JointSensorData& jointSensorData);
  /**
   * @brief loadFromFile loads a MotionFile from a given location
   * @param filename the filepath from which the MotionFile is loaded
   * @return whether loading was successful
   */
  bool loadFromFile(const std::string& filename);
  /**
   * @brief play starts playing of the motion file
   * @return the duration of the motion file [ms]
   */
  int play();
  /**
   * @brief cycle proceeds one cycle in the motion
   * @return the joint values that should be sent for this cycle (whole body)
   */
  JointValues cycle();
  /**
   * @brief isPlaying returns whether the motion is currently playing
   * @return true iff the motion is currently playing
   */
  bool isPlaying() const;
private:
  /**
   * @brief precompile constructs the angles, angle times, stiffnesses and stiffness times
   */
  void precompile();
  /// a reference to the cycle info
  const CycleInfo& cycleInfo_;
  /// a reference to the joint sensor data
  const JointSensorData& jointSensorData_;
  /// the time point when the motion file has been started
  TimePoint startTime_;
  /// the joint values when the motion file has been started
  JointValues startJointValues_;
  /// a sequence of angle frames
  std::vector<std::vector<float>> angles_;
  /// the times (from motion start) [ms] for the angles
  std::vector<int> angleTimes_;
  /// a sequence of stiffnesses
  std::vector<std::vector<float>> stiffnesses_;
  /// the times (from motion start) [ms] for the stiffnesses
  std::vector<int> stiffnessTimes_;
};
