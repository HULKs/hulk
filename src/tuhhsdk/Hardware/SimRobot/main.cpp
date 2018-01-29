#include <SimRobotCore2.h>

#include "SimRobotAdapter.hpp"

extern "C" DLL_EXPORT SimRobot::Module* createModule(SimRobot::Application& simRobot)
{
  return new SimRobotAdapter(simRobot);
}
