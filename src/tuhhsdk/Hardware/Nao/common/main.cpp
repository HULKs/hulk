#include <cstdio>

#include <signal.h>
#include <sys/file.h>
#include <sys/types.h>
#include <unistd.h>

#include "Tools/Backtrace/Backtrace.hpp"

#include "print.h"
#include "tuhh.hpp"

#ifdef NAOV6
#include "Hardware/Nao/v6/Nao6Interface.hpp"
#elif NAOV5
#include "Hardware/Nao/v5/NaoInterface.hpp"
#endif


class PIDFile final
{
public:
  PIDFile(const char* path)
    : fd_(open(path, O_CREAT | O_WRONLY, 0600))
  {
    if (fd_ < 0)
    {
      throw std::runtime_error("Could not create PID file!");
    }
    if (flock(fd_, LOCK_EX | LOCK_NB) == -1)
    {
      close(fd_);
      throw std::runtime_error("Could not lock PID file!");
    }
    dprintf(fd_, "%d\n", getpid());
  }
  ~PIDFile()
  {
    close(fd_);
  }

private:
  int fd_;
};

static volatile int keepRunning = 1;
// The PID file cannot reside in /var/run because that directory is not writable.
static const char* pidFilePath = "/tmp/tuhhNao.pid";

void intHandler(int)
{
  keepRunning = 0;
}

void intErrHandler(int)
{
  std::cout << backtrace() << std::endl;
}

int main()
{
  setvbuf(stderr, nullptr, _IONBF, 0);
  setvbuf(stdout, nullptr, _IOLBF, 0);

  Log(LogLevel::INFO) << "Starting tuhhNao!";

  PIDFile pidFile(pidFilePath);

  // Sig action for sigint and sigterm (normal application shutdown)
  struct sigaction sa;
  sa.sa_handler = &intHandler;
  sigemptyset(&sa.sa_mask);
  sa.sa_flags = SA_RESTART;
  sigaction(SIGINT, &sa, nullptr);
  sigaction(SIGTERM, &sa, nullptr);

  // Sig action for sigsegv and sigabrt (crashes, asserts, ...)
  struct sigaction errAction;
  errAction.sa_handler = &intErrHandler;
  sigemptyset(&errAction.sa_mask);
  sigaction(SIGSEGV, &errAction, nullptr);
  sigaction(SIGABRT, &errAction, nullptr);

  try
  {
    sigset_t mask;
    sigemptyset(&mask);
    NaoInterface robotInterface;
    TUHH tuhh(robotInterface);
    while (keepRunning)
    {
      sigsuspend(&mask);
    }
    Log(LogLevel::INFO) << "Received signal, shutting application down!";
  }
  catch (const std::exception& e)
  {
    Log(LogLevel::ERROR) << "Exception in NaoInterface or TUHH:";
    Log(LogLevel::ERROR) << e.what();
    return EXIT_FAILURE;
  }
  catch (...)
  {
    Log(LogLevel::ERROR)
        << "Unknown exception in NaoInterface or TUHH (which means it could be anywhere)!";
    return EXIT_FAILURE;
  }

  return EXIT_SUCCESS;
}
