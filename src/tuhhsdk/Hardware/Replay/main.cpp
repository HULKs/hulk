#ifdef _WIN32
#include <thread>
#endif

#include <signal.h>

#include "tuhh.hpp"
#include "ReplayInterface.hpp"
#include "print.h"

static volatile int keepRunning = 1;

void intHandler(int)
{
  keepRunning = 0;
}


int main(int argc, char *argv[])
{
  if (argc != 2)
  {
    Log(LogLevel::ERROR) << "Usage: tuhhReplay <file containing replay data>";
    return EXIT_FAILURE;
  }

#ifndef _WIN32
  struct sigaction sa;
  sa.sa_handler = &intHandler;
  sigemptyset(&sa.sa_mask);
  sa.sa_flags = SA_RESTART;
  sigaction(SIGINT, &sa, nullptr);
  sigaction(SIGTERM, &sa, nullptr);
#else
  signal(SIGINT, intHandler);
#endif

  std::shared_ptr<ReplayInterface> robotInterface;
  try
  {
    robotInterface = std::make_shared<ReplayInterface>(argv[1]);
  }
  catch (const std::exception& e)
  {
    Log(LogLevel::ERROR) << "Exception in ReplayInterface constructor:";
    Log(LogLevel::ERROR) << e.what();
    return EXIT_FAILURE;
  }

  try
  {
#ifndef _WIN32
    sigset_t mask;
    sigemptyset(&mask);
#endif
    TUHH tuhh(*robotInterface);
    while (keepRunning)
    {
#ifndef _WIN32
      sigsuspend(&mask);
#else
      std::this_thread::sleep_for(std::chrono::milliseconds(500));
#endif
    }
    Log(LogLevel::INFO) << "Received signal, shutting application down!";
  }
  catch (const std::exception& e)
  {
    Log(LogLevel::ERROR) << "Exception in TUHH:";
    Log(LogLevel::ERROR) << e.what();
    return EXIT_FAILURE;
  }
  catch (...)
  {
    Log(LogLevel::ERROR) << "Unknown exception in TUHH (which means it could be anywhere)!";
    return EXIT_FAILURE;
  }

  return EXIT_SUCCESS;
}
