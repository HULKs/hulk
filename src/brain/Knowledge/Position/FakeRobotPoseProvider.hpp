#pragma once

#include "Framework/Module.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FakeImageData.hpp"
#include "Data/RobotPosition.hpp"


class Brain;

/**
 * @brief The FakeRobotPoseProvider class
 */
class FakeRobotPoseProvider : public Module<FakeRobotPoseProvider, Brain>, public Uni::To
{
public:
  /// the name of this module
  ModuleName name = "FakeRobotPoseProvider";
  /**
   * @brief the constructor of this module
   * @param manager the module manager interface
   */
  FakeRobotPoseProvider(const ModuleManagerInterface& manager);
  /**
   * @brief cycle writes the fake robot pose from the robot interface to the
   *        RobotPosition production
   */
  void cycle();
  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the resulting Uni::Value
   */
  void toValue(Uni::Value& value) const;

private:
  /// a dependency to ensure that there is fake data availabe before this module runs
  const Dependency<FakeImageData> fakeImageData_;
  /// some details about the cycle time
  const Dependency<CycleInfo> cycleInfo_;
  /// the fake production of this module
  Production<RobotPosition> fakeRobotPose_;
  /// the pose of the last cylce
  Pose lastPose_;
  /// the pose of this cyle
  Pose pose_;
  /// a timestamp of the last mayor pose change
  TimePoint lastTimeJumped_;
  /// updates the lastTimeJumped_ member
  void updateLastTimeJumped();
};
