#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/JointSensorData.hpp"
#include "Hardware/Clock.hpp"
#include "Hardware/Definitions.hpp"
#include "Motion/Utils/MotionFile/MotionFile.hpp"
#include <vector>


class MotionFilePlayer : private MotionFile
{
public:
  /**
   * @struct JointValues
   */
  struct JointValues
  {
    /// all the joint angles in a frame
    JointsArray<float> angles{};
    /// all the joint stiffnesses in a frame
    JointsArray<float> stiffnesses{};
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
   * @brief stop stops playing of the motion file
   */
  void stop();
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
  Clock::time_point startTime_;
  /// the joint values when the motion file has been started
  JointValues startJointValues_;
  /// a sequence of angle frames
  std::vector<JointsArray<float>> angles_;
  /// the times (from motion start) [ms] for the angles
  std::vector<int> angleTimes_;
  /// a sequence of stiffnesses
  std::vector<JointsArray<float>> stiffnesses_;
  /// the times (from motion start) [ms] for the stiffnesses
  std::vector<int> stiffnessTimes_;
};
