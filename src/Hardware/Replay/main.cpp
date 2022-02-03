#include "Framework/Log/Log.hpp"
#include "Framework/tuhh.hpp"
#include "Hardware/Replay/ReplayInterface.hpp"
#include "Tools/Backtrace/Backtrace.hpp"
#include <csignal>

static volatile bool keepRunning = true;

void intHandler(int /*unused*/)
{
  keepRunning = false;
}

void intErrHandler(int /*unused*/)
{
  std::cout << backtrace() << std::endl;
}

int main(int argc, char* argv[])
{
  if (argc != 2)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR) << "Usage: tuhhReplay <file containing replay data>";
    return EXIT_FAILURE;
  }

  struct sigaction sa
  {
  };
  sa.sa_handler = &intHandler;
  sigemptyset(&sa.sa_mask);
  sa.sa_flags = SA_RESTART;
  sigaction(SIGINT, &sa, nullptr);
  sigaction(SIGTERM, &sa, nullptr);

  // Sig action for sigsegv and sigabrt (crashes, asserts, ...)
  struct sigaction errAction
  {
  };
  errAction.sa_handler = &intErrHandler;
  sigemptyset(&errAction.sa_mask);
  sigaction(SIGSEGV, &errAction, nullptr);
  sigaction(SIGABRT, &errAction, nullptr);

  std::shared_ptr<ReplayInterface> robotInterface;
  try
  {
    robotInterface = std::make_shared<ReplayInterface>(argv[1]);
  }
  catch (const std::exception& e)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR) << "Exception in ReplayInterface constructor:";
    Log<M_TUHHSDK>(LogLevel::ERROR) << e.what();
    return EXIT_FAILURE;
  }

  try
  {
    sigset_t mask;
    sigemptyset(&mask);
    TUHH tuhh(*robotInterface);
    while (keepRunning)
    {
      std::this_thread::sleep_for(std::chrono::milliseconds(500));
    }
    Log<M_TUHHSDK>(LogLevel::INFO) << "Received signal, shutting application down";
  }
  catch (const std::exception& e)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR) << "Exception in TUHH:";
    Log<M_TUHHSDK>(LogLevel::ERROR) << e.what();
    return EXIT_FAILURE;
  }
  catch (...)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR)
        << "Unknown exception in TUHH (which means it could be anywhere)";
    return EXIT_FAILURE;
  }

  return EXIT_SUCCESS;
}
