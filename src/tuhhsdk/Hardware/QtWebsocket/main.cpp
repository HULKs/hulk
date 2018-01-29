#include <thread>

#include "tuhh.hpp"
#include "print.h"
#include "QtWebsocketInterface.hpp"

int main(int argc, char** argv)
{
  Log(LogLevel::INFO) << "Starting tuhhQtWebsocket!";
  std::shared_ptr<QtWebsocketInterface> robotInterface(new QtWebsocketInterface(argc, argv));

  try
  {
    TUHH tuhh(*robotInterface);
    while (true)
    {
      std::this_thread::sleep_for(std::chrono::milliseconds(500));
    }
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
