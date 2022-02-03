#pragma once

#include "Data/ActionCommand.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/Poses.hpp"
#include "Data/SitDownOutput.hpp"
#include "Framework/Module.hpp"
#include "Motion/Utils/MotionFile/MotionFilePlayer.hpp"
#include <string>

class Motion;

class SitDown : public Module<SitDown, Motion>
{
public:
  ModuleName name__{"SitDown"};
  /**
   * @brief SitDown initializes members and loads motion files
   * @param manager a reference to motion
   */
  explicit SitDown(const ModuleManagerInterface& manager);
  /**
   * @brief cycle checks for a new command and initiates a sit down motion if needed
   */
  void cycle() override;

private:
  /**
   * @brief Status is an enum to specify the status of the SitDown module
   */
  enum class Status
  {
    IDLE,
    SITTING_DOWN,
    DONE
  };
  const Dependency<ActionCommand> actionCommand_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<JointSensorData> jointSensorData_;
  /// a reference to the motion activation of last cycle
  const Reference<MotionActivation> motionActivation_;
  const Dependency<Poses> poses_;

  Production<SitDownOutput> sitDownOutput_;

  /// name of motion file containing the needed motion for sitting down
  const Parameter<std::string> sitDownMotionFile_;

  /// state of the SitDown-module
  Status status_;
  /// motion-object for whole sit down motion
  MotionFilePlayer sitDownMotion_;
};
