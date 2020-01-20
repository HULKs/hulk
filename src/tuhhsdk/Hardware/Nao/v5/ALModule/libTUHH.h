/**
 * @file libTUHH.h
 * @brief File providing the sensor and actuator communication.
 * @author <a href="mailto:stefan.kaufmann@tu-harburg.de">Stefan Kaufmann</a>
 * @author <a href="mailto:nicolas.riebesel@tuhh.de">Nicolas Riebesel</a>
 * @author <a href="mailto:oliver.tretau@tuhh.de">Oliver Tretau</a>
 *
 * Two classes are introduces in order to communicate with the AL framework.
 */

#ifndef LIBTUHH_H
#define LIBTUHH_H

#include <chrono>
#include <time.h>

#include <boost/interprocess/shared_memory_object.hpp>
#include <boost/interprocess/mapped_region.hpp>
#include <boost/shared_ptr.hpp>
#include <boost/signals.hpp>
#include <boost/bind.hpp>
#include <alcommon/albroker.h>
#include <alcommon/alproxy.h>
#include <alcommon/almodule.h>
#include <alproxies/dcmproxy.h>

#include "Definitions/keys.h"
#include "Hardware/Nao/common/SMO.h"

#include "Hardware/Nao/common/BatteryDisplay.hpp"

/**
 * @class libTUHH
 * @brief The libTUHH class is providing the interface used for communicating
 *        with AL Libraries over a shared momery structure.
 * @author <a href="mailto:stefan.kaufmann@tu-harburg.de">Stefan Kaufmann</a>
 * @author <a href="mailto:nicolas.riebesel@tuhh.de">Nicolas Riebesel</a>
 * @author <a href="mailto:oliver.tretau@tuhh.de">Oliver Tretau</a>
 *
 * This class provides the ALBroker and the ALMemoryProxy which are mendatory
 * in order to communicate with the Aldebaran framework. Detailed information
 * might be available <a href="http://doc.aldebaran.com/2-1/dev/cpp/index.html">here<a/>.
 */
class libTUHH : public AL::ALModule
{
public:
  /**
   * @brief libTUHH function which takes care of some initialization processes
   * @param broker a shared pointer on AL::ALBroker
   * @param name the name of the module
   */
  libTUHH(boost::shared_ptr<AL::ALBroker> broker, const std::string& name);

  /**
   * @brief ~libTUHH closes the shared memory
   */
  ~libTUHH();

  /**
   * @brief init is called by naoqi when libTUHH is registered
   */
  void init();

  /**
   * @brief exit is called by naoqi when libTUHH is unregistered
   */
  void exit();

  /**
   * @brief postCycle function which writes values from AL to shared memory
   */
  void postCycle();

  /**
   * @brief preCycle function which reads shared memory and pass commands to AL
   */
  void preCycle();

  /**
   * @brief staticPreCycle static method to call preCycle on the only instance
   */
  static void staticPreCycle();

  /**
   * @brief staticPostCycle static method to call postCycle on the only instance
   */
  static void staticPostCycle();
private:

  float* jointSensor[keys::joints::JOINTS_MAX];      ///< Sensor values of all joints
  float* jointCurrent[keys::joints::JOINTS_MAX];     ///< Current values of all joints
  float* jointTemperature[keys::joints::JOINTS_MAX]; ///< Temperature values of all joints
  float* jointStatus[keys::joints::JOINTS_MAX];      ///< Status values of all joints

  float* switches[keys::sensor::SWITCH_MAX]; ///< All switch key values
  float* imu[keys::sensor::IMU_MAX];         ///< All imu key values
  float* fsrLeft[keys::sensor::FSR_MAX];     ///< All left Force Sensitive Resistors (FSR) key values
  float* fsrRight[keys::sensor::FSR_MAX];    ///< All right Force Sensitive Resistors (FSR) key values
  float* sonar[keys::sensor::SONAR_MAX];     ///< All sonar key values
  float* battery[keys::sensor::BATTERY_MAX]; ///< All battery key values

  /// the circular head LEDs for the battery display
  float batteryLEDs[keys::led::HEAD_MAX];

  // Shared memory
  struct SHMRemover {
    SHMRemover()
    {
      boost::interprocess::shared_memory_object::remove(SMO::shmName);
    }
    ~SHMRemover()
    {
      boost::interprocess::shared_memory_object::remove(SMO::shmName);
    }
  } remover_;
  boost::interprocess::shared_memory_object segment_;  ///< Shared memory segment
  boost::interprocess::mapped_region region_;          ///< Shared memory mapping
  SharedBlock* shmBlock_;                              ///< Shared memory block

  boost::shared_ptr<AL::ALBroker> broker;          ///< Broker that started this module
  boost::shared_ptr<AL::DCMProxy> dcmProxy;        ///< Device Communication Manager Proxy for AL communication
  boost::signals::connection preCycleConnection;   ///< Signal for binding to preCycle
  boost::signals::connection postCycleConnection;  ///< Signal for binding to postCycle

  float previousChestButton;            ///< Previous value of the chest button sensor
  std::chrono::time_point<std::chrono::system_clock> previousChestButtonTime; ///< The time when the chest button has been pressed
  bool sentChestButton;                 ///< Whether a chest button event has been sent

  BatteryDisplay batteryDisplay;

  AL::ALValue angleCommand;
  AL::ALValue stiffnessCommand;
  AL::ALValue ledCommand;
  AL::ALValue batteryCommand;
  AL::ALValue sonarCommand;

  bool droppedFrame;

  static libTUHH* instance;
};

#endif // LIBTUHH_H
