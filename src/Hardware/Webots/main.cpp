#include "Framework/Log/Log.hpp"
#include "Framework/tuhh.hpp"
#include "Hardware/Webots/WebotsInterface.hpp"
#include <csignal>
#include <memory>

// NOLINTNEXTLINE(cppcoreguidelines-avoid-non-const-global-variables)
static std::unique_ptr<WebotsInterface> robotInterface;

void signalHandler([[maybe_unused]] int signal)
{
  robotInterface->terminate();
}

int main()
{
  Log<M_TUHHSDK>(LogLevel::INFO) << "Starting webots!";

  struct sigaction signalAction
  {
  };
  signalAction.sa_handler = &signalHandler;
  sigemptyset(&signalAction.sa_mask);
  signalAction.sa_flags = SA_RESTART;
  sigaction(SIGINT, &signalAction, nullptr);
  sigaction(SIGTERM, &signalAction, nullptr);

  try
  {
    robotInterface = std::make_unique<WebotsInterface>();
    TUHH tuhh{*robotInterface};
    robotInterface->waitForTermination();
  }
  catch (const std::exception& e)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR) << "Exception in WebotsInterface or TUHH:";
    Log<M_TUHHSDK>(LogLevel::ERROR) << e.what();
    return EXIT_FAILURE;
  }
  catch (...)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR)
        << "Unknown exception in WebotsInterface or TUHH (which means it could be anywhere)";
    return EXIT_FAILURE;
  }

  return EXIT_SUCCESS;
}
