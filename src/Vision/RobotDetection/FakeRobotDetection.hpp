#pragma once

#include "Data/CycleInfo.hpp"
#include "Data/FakeImageData.hpp"
#include "Data/RobotData.hpp"
#include "Framework/Module.hpp"


class Brain;

/**
 * @brief The FakeRobotDetection class
 */
class FakeRobotDetection : public Module<FakeRobotDetection, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"FakeRobotDetection"};
  /**
   * @brief the constructor of this module
   * @param manager the module manager interface
   */
  FakeRobotDetection(const ModuleManagerInterface& manager);
  /**
   * @brief cycle writes the (faked) position of other robots to the production
   */
  void cycle();

private:
  /// a dependency to ensure that there is fake data availabe before this module runs
  const Dependency<FakeImageData> fakeImageData_;
  /// the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// the production of this module: mainly the detected robots in robot coordinates
  Production<RobotData> fakeRobotData_;
};
