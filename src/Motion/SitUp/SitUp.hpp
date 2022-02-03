#pragma once

#include "Data/ActionCommand.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/MotionActivation.hpp"
#include "Data/Poses.hpp"
#include "Data/SitUpOutput.hpp"
#include "Framework/Module.hpp"
#include "Motion/Utils/MotionFile/MotionFilePlayer.hpp"
#include <string>

class Motion;

class SitUp : public Module<SitUp, Motion>
{
public:
  ModuleName name__{"SitUp"};
  /**
   * @brief SitUp initializes members and loads motion files
   * @param manager a reference to motion
   */
  explicit SitUp(const ModuleManagerInterface& manager);
  /**
   * @brief cycle checks for a new command and initiates a sit up motion if needed
   */
  void cycle() override;

private:
  /**
   * @brief State is an enum to specify the state of the SitUp module
   */
  enum class State
  {
    IDLE,
    SITTING_UP,
    DONE
  };

  const Dependency<ActionCommand> actionCommand_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<JointSensorData> jointSensorData_;
  /// a reference to the motion activation of last cycle
  const Reference<MotionActivation> motionActivation_;
  const Dependency<Poses> poses_;

  Production<SitUpOutput> sitUpOutput_;

  /// name of motion file containing the needed motion for sitting up
  const Parameter<std::string> sitUpMotionFile_;

  /// state of the SitUp-module
  State state_;
  /// motion-object for whole sit up motion
  MotionFilePlayer sitUpMotion_;
};
