#include <string>

#ifdef NAO
#include <mntent.h>
#endif

#include <fftw3.h>

#include "Modules/Debug/FileTransport.h"
#if !defined(SIMROBOT) || defined(WIN32)
#include "Modules/Debug/TCPTransport.h"
#else
#include "Modules/Debug/UnixSocketTransport.hpp"
#endif
#include "Modules/NaoProvider.h"
#include "Modules/Poses.h"

#include "print.h"

#include "tuhh.hpp"

#include "Tools/Storage/XPMImage.hpp"

TUHH::TUHH(RobotInterface& robotInterface)
  : interface_(robotInterface)
  , config_(interface_.getFileRoot())
  , sharedObjectManager_(debug_, config_, interface_)
{
  print("Start init of tuhh", LogLevel::FANCY);
  XPMImage::init();
  // load configuration file
  config_.mount("tuhhSDK.base", "sdk.json", ConfigurationType::HEAD);
#ifdef SIMROBOT
  config_.setLocationName("simrobot");
#else
  // set location so the next configuration files will be loaded from there
  config_.setLocationName(config_.get("tuhhSDK.base", "location").asString());
#endif

  NaoInfo info;
  interface_.getNaoInfo(config_, info);
  // set NAO version and name to enable configuration files to be loaded nao specific
  config_.setNaoHeadName(info.headName);
  config_.setNaoBodyName(info.bodyName);

  tuhhprint::print("About to configure interface", LogLevel::FANCY);
  // At this point, all configuration specifiers (location, body name, head name) will be set correctly.
  interface_.configure(config_);

  Poses::init(interface_.getFileRoot());

  NaoProvider::init(config_, info);

  LogLevel const ll = tuhhprint::getLogLevel(config_.get("tuhhSDK.base", "loglevel").asString());
  tuhhprint::print("The current loglevel is " + tuhhprint::preString[(int)ll], LogLevel::INFO);
  tuhhprint::setLogLevel(ll);

  if (config_.get("tuhhSDK.base", "local.enableFileTransport").asBool())
  {
    std::string fileTransportRoot = interface_.getFileRoot();

#ifdef NAO
    mntent* ent;
    mntent dummy;
    char* buf = new char[4096];
    FILE* aFile = setmntent("/proc/mounts", "r");

    if (aFile != nullptr)
    {
      while (nullptr != (ent = getmntent_r(aFile, &dummy, buf, 4096)))
      {
        std::string fsname(ent->mnt_fsname);
        if (fsname == "/dev/sdb1")
        {
          fileTransportRoot = std::string(ent->mnt_dir) + "/";
          tuhhprint::print("Will use " + fileTransportRoot + " as FileTransport directory!", LogLevel::FANCY);
          break;
        }
      }
      endmntent(aFile);
    }
    else
    {
      tuhhprint::print("Could not get mountpoints for FileTransport directory!", LogLevel::ERROR);
    }
    delete[] buf;
#endif

    debug_.addTransport(std::make_shared<FileTransport>(debug_, config_, fileTransportRoot));
  }

#if !defined(SIMROBOT) || defined(WIN32)
  const std::uint16_t basePort = config_.get("tuhhSDK.base", "network.basePort").asInt();

  if (config_.get("tuhhSDK.base", "network.enableConfiguration").asBool())
  {
    nc_ = std::make_unique<NetworkConfig>(basePort + 2, config_);
    nc_->run();
  }

  if (config_.get("tuhhSDK.base", "network.enableDebugTCPTransport").asBool())
  {
    debug_.addTransport(std::make_shared<TCPTransport>(basePort + 1, debug_));
  }
#else
  usc_ = std::make_unique<UnixSocketConfig>(config_.get("tuhhSDK.base", "local.unixSocketDirectory").asString() + info.headName + "/config", config_);
  usc_->run();
  debug_.addTransport(
      std::make_shared<UnixSocketTransport>(config_.get("tuhhSDK.base", "local.unixSocketDirectory").asString() + info.headName + "/debug", debug_));
#endif

  sharedObjectManager_.start();

#ifndef SIMROBOT
  if (config_.get("tuhhSDK.base", "network.enableAliveness").asBool())
  {
    at_ = std::make_unique<AlivenessTransmitter>(basePort, info);
    at_->startTransmitting();
  }
#endif

  // see SoftBank documentation if you want to know where 68 comes from
  interface_.setSonar(68);

  debug_.start();
}

TUHH::~TUHH()
{
  debug_.stop();
  sharedObjectManager_.stop();
#if !defined(SIMROBOT)
  nc_ = nullptr;
  at_ = nullptr;
#elif defined(WIN32)
  nc_ = nullptr;
#else
  usc_ = nullptr;
#endif
  tuhhprint::setLogLevel(LogLevel::VERBOSE);
  fftw_cleanup();
  // This makes sure that all transports are destroyed before the Debug destructor is invoked.
  // It is necessary because transports have a reference to Debug which will become invalid then.
  debug_.removeAllTransports();
}
