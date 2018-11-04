#pragma once

#include "Framework/Module.hpp"

class Motion;

/**
 * The only purpose of the MotionSleep module is to keep the motion thread busy in
 * moduleSetup_replay, while there is no other active motion module.
 * @author Georg Felbinger
 */
class MotionSleep : public Module<MotionSleep, Motion>
{
public:
  /// the name of this module
  ModuleName name = "MotionSleep";

  MotionSleep(const ModuleManagerInterface& manager)
    : Module(manager)
  {
  }

  void cycle()
  {
    std::this_thread::sleep_for(std::chrono::milliseconds(10));
  }
};
