#include "Framework/tuhh.hpp"
#include "Framework/Debug/FileTransport.h"
#include "Framework/Log/Log.hpp"
#include "Tools/Storage/XPM/XPMImage.hpp"
#include <fftw3.h>
#include <string>

#ifdef HULK_UNIX_SOCKET
#include "Framework/Debug/UnixSocketTransport.hpp"
#else
#include "Framework/Debug/TCPTransport.h"
#endif

TUHH::TUHH(RobotInterface& robotInterface)
  : interface_(robotInterface)
  , config_(interface_.getFileRoot())
  , sharedObjectManager_(debug_, config_, interface_)
{
  Log<M_TUHHSDK>(LogLevel::FANCY) << "Start init of tuhh";
  XPMImage::init();
  // load configuration file
  config_.mount("tuhhSDK.base", "sdk.json", ConfigurationType::HEAD);
#if defined(HULK_TARGET_SimRobot)
  config_.setLocationName("SimRobot");
#elif defined(HULK_TARGET_Webots)
  config_.setLocationName("Webots");
#else
  // set location so the next configuration files will be loaded from there
  config_.setLocationName(config_.get("tuhhSDK.base", "location").asString());
#endif

  Log<M_TUHHSDK>(LogLevel::FANCY) << "About to configure interface";
  // At this point, all configuration specifiers (location, body name, head name) will be set
  // correctly.
  interface_.configure(config_);

  LogLevel const ll =
      Log<M_TUHHSDK>::getLogLevel(config_.get("tuhhSDK.base", "loglevel").asString());
  Log<M_TUHHSDK>(LogLevel::INFO) << "The current LogLevel is " << Log<M_TUHHSDK>::getPreString(ll);
  Log<M_TUHHSDK>::setLogLevel(ll);

  if (config_.get("tuhhSDK.base", "local.enableFileTransport").asBool())
  {
    std::string fileTransportRoot = interface_.getDataRoot();
    debug_.addTransport(std::make_shared<FileTransport>(debug_, config_, fileTransportRoot));
  }

#ifdef HULK_UNIX_SOCKET
  unixSocketConfig_ = std::make_unique<UnixSocketConfig>(
      config_.get("tuhhSDK.base", "local.unixSocketDirectory").asString() +
          interface_.getRobotInfo().headName + "/config",
      config_);
  unixSocketConfig_->run();
  debug_.addTransport(std::make_shared<UnixSocketTransport>(
      config_.get("tuhhSDK.base", "local.unixSocketDirectory").asString() +
          interface_.getRobotInfo().headName + "/debug",
      debug_));
#else
  {
    const std::uint16_t basePort = config_.get("tuhhSDK.base", "network.basePort").asInt32();

    if (config_.get("tuhhSDK.base", "network.enableConfiguration").asBool())
    {
      networkConfig_ = std::make_unique<NetworkConfig>(basePort + 2, config_);
      networkConfig_->run();
    }

    if (config_.get("tuhhSDK.base", "network.enableDebugTCPTransport").asBool())
    {
      debug_.addTransport(std::make_shared<TCPTransport>(basePort + 1, debug_));
    }
  }
#endif

  sharedObjectManager_.start();

  debug_.start();
}

TUHH::~TUHH()
{
  debug_.stop();
  sharedObjectManager_.stop();
  Log<M_TUHHSDK>::setLogLevel(LogLevel::VERBOSE);
  fftw_cleanup();
  // This makes sure that all transports are destroyed before the Debug destructor is invoked.
  // It is necessary because transports have a reference to Debug which will become invalid then.
  debug_.removeAllTransports();
}
