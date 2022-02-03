#include "Hardware/SimRobot/SimRobotAdapter.hpp"
#include "Tools/Backtrace/Backtrace.hpp"
#include <SimRobotCore2.h>
#include <csignal>

void intErrHandler([[maybe_unused]] int signal)
{
  std::cout << backtrace() << std::endl;
}

extern "C" DLL_EXPORT SimRobot::Module* createModule(SimRobot::Application& simRobot)
{
  // Sig action for sigsegv and sigabrt (crashes, asserts, ...)
  struct sigaction errAction
  {
  };
  errAction.sa_handler = &intErrHandler;
  sigemptyset(&errAction.sa_mask);
  sigaction(SIGSEGV, &errAction, nullptr);
  sigaction(SIGABRT, &errAction, nullptr);

  return new SimRobotAdapter(simRobot);
}
