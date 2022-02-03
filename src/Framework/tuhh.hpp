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

#include "Framework/Configuration/Configuration.h"
#ifdef HULK_UNIX_SOCKET
#include "Framework/Configuration/UnixSocketConfig.hpp"
#else
#include "Framework/Configuration/NetworkConfig.hpp"
#endif
#include "Framework/Debug/Debug.h"

#include "Hardware/RobotInterface.hpp"

#include "Framework/SharedObjectManager.hpp"

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
  explicit TUHH(RobotInterface& robotInterface);
  TUHH(const TUHH&) = delete;
  TUHH(TUHH&&) = delete;
  TUHH& operator=(const TUHH&) = delete;
  TUHH& operator=(TUHH&&) = delete;
  /**
   * @brief ~TUHH stops all threads and destroys almost all objects
   */
  ~TUHH();

private:
  RobotInterface& interface_;

  Configuration config_;
  Debug debug_;

#ifdef HULK_UNIX_SOCKET
  std::unique_ptr<UnixSocketConfig> unixSocketConfig_;
#else
  std::unique_ptr<NetworkConfig> networkConfig_;
#endif
  SharedObjectManager sharedObjectManager_;
};
