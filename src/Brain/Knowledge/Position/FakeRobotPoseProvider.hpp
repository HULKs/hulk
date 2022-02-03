#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/FakeImageData.hpp"
#include "Data/RobotPosition.hpp"
#include "Framework/Module.hpp"

class Brain;

/**
 * @brief The FakeRobotPoseProvider class
 */
class FakeRobotPoseProvider : public Module<FakeRobotPoseProvider, Brain>, public Uni::To
{
public:
  /// the name of this module
  ModuleName name__{"FakeRobotPoseProvider"};
  /**
   * @brief the constructor of this module
   * @param manager the module manager interface
   */
  explicit FakeRobotPoseProvider(const ModuleManagerInterface& manager);
  /**
   * @brief cycle writes the fake robot pose from the robot interface to the
   *        RobotPosition production
   */
  void cycle() override;
  /**
   * @brief toValue converts this to a Uni::Value
   * @param value the resulting Uni::Value
   */
  void toValue(Uni::Value& value) const override;

private:
  /// whether the own pose should be mirrored (useful in simrobot for enemy team)
  const Parameter<bool> mirrorFakePose_;
  /// some details about the cycle time
  const Dependency<CycleInfo> cycleInfo_;
  /// the fake production of this module
  Production<RobotPosition> fakeRobotPose_;
  /// the pose of the last cylce
  Pose lastPose_;
  /// the pose of this cyle
  Pose pose_;
  /// a timestamp of the last mayor pose change
  Clock::time_point lastTimeJumped_;
  /// updates the lastTimeJumped_ member
  void updateLastTimeJumped();
};
