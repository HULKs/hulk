#pragma once

#include "Data/FakeImageData.hpp"
#include "Data/RobotData.hpp"
#include "Data/CycleInfo.hpp"
#include "Framework/Module.hpp"


class Brain;

/**
 * @brief The RobotDetection class
 */
class RobotDetection : public Module<RobotDetection, Brain>
{
public:
  /// the name of this module
  ModuleName name = "RobotDetection";
  /**
   * @brief the constructor of this module
   * @param manager the module manager interface
   */
  RobotDetection(const ModuleManagerInterface& manager);
  /**
   * @brief cycle writes the (faked) position of other robots to the production
   */
  void cycle();

private:
  /// the cycle info
  const Dependency<CycleInfo> cycleInfo_;
  /// the production of this module: mainly the detected robots in robot coordinates
  Production<RobotData> robotData_;
};
