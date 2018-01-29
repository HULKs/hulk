/**
 * @mainpage Documentation for the tuhhSDK
 * @file tuhh.hpp
 * @brief This file provides the main Software Development Kit.
 * @author <a href="mailto:stefan.kaufmann@tuhh.de">Stefan Kaufmann</a>
 * @author <a href="mailto:nicolas.riebesel@tuhh.de">Nicolas Riebesel</a>
 * @author <a href="mailto:oliver.tretau@tuhh.de">Oliver Tretau</a>
 *
 * The tuhhSDK consists of multiple classes and a lot of functions. It is
 * developed by students of the TU Hamburg-Harburg. The SDK is provided as
 * Open Source, so you can look into the functions, make changes or extend the
 * SDK. If something is not documented well, please contact one of the authors.
 */

#pragma once

#include <memory>

#include "Modules/Configuration/Configuration.h"
#if !defined(SIMROBOT) || defined(WIN32)
#include "Modules/Configuration/NetworkConfig.hpp"
#else
#include "Modules/Configuration/UnixSocketConfig.hpp"
#endif
#include "Modules/Debug/Debug.h"
#ifndef SIMROBOT
#include "Modules/Network/AlivenessTransmitter.h"
#endif
#include "Hardware/RobotInterface.hpp"

#include "SharedObjectManager.hpp"


/**
 * @class TUHH is the main class of the complete software system
 * It instantiates the threads (currently via the SharedObject class) that run the modules.
 */
class TUHH
{
public:
  /**
   * @brief TUHH intializes some important static classes and starts threads
   * @param robotInterface an interface to communicate with the hardware of the NAO
   */
  TUHH(RobotInterface& robotInterface);
  /**
   * @brief ~TUHH stops all threads and destroys almost all objects
   */
  ~TUHH();

private:
  RobotInterface& interface_;

  // callback list
  NaoSensorData sensors_;

  Configuration config_;
  Debug debug_;
#if !defined(SIMROBOT)
  std::unique_ptr<AlivenessTransmitter> at_;
  std::unique_ptr<NetworkConfig> nc_;
#elif defined(WIN32)
  std::unique_ptr<NetworkConfig> nc_;
#else
  std::unique_ptr<UnixSocketConfig> usc_;
#endif
  SharedObjectManager sharedObjectManager_;
};
