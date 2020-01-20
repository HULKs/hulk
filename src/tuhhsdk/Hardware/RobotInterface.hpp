#pragma once

#include <array>
#include <string>
#include <vector>

#include "AudioInterface.hpp"
#include "CameraInterface.hpp"
#include "Definitions/keys.h"
#include "FakeDataInterface.hpp"
#include "Tools/Time.hpp"

/**
 * @enum callback_event
 * @brief The callback_event enum provides types of callback events pushed over
 *        the shared memory.
 */
enum callback_event
{
  CE_CHESTBUTTON_DOUBLE,
  CE_CHESTBUTTON_SIMPLE,
  __CE_MAX
};

struct NaoSensorData
{
  // joint information
  /// Sensor values of all joints
  std::array<float, keys::joints::JOINTS_MAX> jointSensor;
  /// Current values of all joints
  std::array<float, keys::joints::JOINTS_MAX> jointCurrent;
  /// Temperature values of all joints
  std::array<float, keys::joints::JOINTS_MAX> jointTemperature;
  /// Status values of all joints
  std::array<float, keys::joints::JOINTS_MAX> jointStatus;

  // sensor information
  /// All switch key values
  std::array<float, keys::sensor::SWITCH_MAX> switches;
  /// All imu key values
  std::array<float, keys::sensor::IMU_MAX> imu;
  /// All left Force Sensitive Resistors (FSR) key values
  std::array<float, keys::sensor::FSR_MAX> fsrLeft;
  /// All right Force Sensitive Resistors (FSR) key values
  std::array<float, keys::sensor::FSR_MAX> fsrRight;
  /// All sonar key values
  std::array<float, keys::sensor::SONAR_MAX> sonar;
  /// All battery key values
  std::array<float, keys::sensor::BATTERY_MAX> battery;

  /// Real time when sensor values were sampled
  TimePoint time;

  /// List of button events that were detected this cycle
  std::vector<callback_event> buttonCallbackList;
};

enum class NaoVersion
{
  /// some unknown or unsupported version
  UNKNOWN,
  /// version 3.3 head or body
  V3_3,
  /// version 4 head or body
  V4,
  /// version 5 head or body
  V5,
  /// version 6 head or body
  V6
};

struct NaoInfo : public Uni::To
{
  /// the version of the body
  NaoVersion bodyVersion;
  /// the version of the head
  NaoVersion headVersion;
  /// a body name, e.g. tuhhnao11
  std::string bodyName;
  /// a head name, e.g. tuhhnao03
  std::string headName;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["bodyVersion"] << static_cast<int>(bodyVersion);
    value["headVersion"] << static_cast<int>(headVersion);
    value["bodyName"] << bodyName;
    value["headName"] << headName;
  }
};

class Configuration;

class RobotInterface
{
public:
  /**
   * @brief ~RobotInterface a virtual constructor for polymorphism
   */
  virtual ~RobotInterface() = default;
  /**
   * @brief configure does things that require configuration files to be loaded for the correct
   * location / NAO
   * @note This method should be called exactly once.
   * @param config a reference to the Configuration instance
   * @param naoInfo head and body version/name
   */
  virtual void configure(Configuration& config, NaoInfo& naoInfo) = 0;
  /**
   * @brief setJointAngles sets the joint angles for the current cycle
   * @param angles the values of all joint angles
   */
  virtual void setJointAngles(const std::vector<float>& angles) = 0;
  /**
   * @brief setJointStiffnesses sets the joint stiffnesses for the current cycle
   * @param stiffnesses the values of all joint stiffnesses
   */
  virtual void setJointStiffnesses(const std::vector<float>& stiffnesses) = 0;
  /**
   * @brief setLEDs sets the LED colors and/or brightnesses
   * @param leds the values of all LEDs
   */
  virtual void setLEDs(const std::vector<float>& leds) = 0;
  /**
   * @brief setSonar sets the value of the sonar actuator
   * @param sonar the value of the sonar actuator (see Soft Bank documentation)
   */
  virtual void setSonar(const float sonar) = 0;
  /**
   * @brief waitAndReadSensorData copies sensor values
   * @param sensors is filled with the sensor data from the last cycle
   * @return duration between the last and current received sensor data
   */
  virtual float waitAndReadSensorData(NaoSensorData& data) = 0;
  /**
   * @brief getFileRoot returns a path to a directory that contains all files for our program
   * @return a path
   */
  virtual std::string getFileRoot() = 0;
  /**
   * @brief getDataRoot returns a path where files can be stored during the game, e.g. fileTransport
   * or ReplayRecorder data
   * @return a path
   */
  virtual std::string getDataRoot() = 0;
  /**
   * @brief getNaoInfo copies the hardware identification
   * @param config a reference to the Configuration instance
   * @param info is filled with the body/head version and name
   */
  virtual void getNaoInfo(Configuration& config, NaoInfo& info) = 0;
  /**
   * @brief getCamera provides access to the cameras of the robot
   * @return a reference to the requested camera
   */
  virtual CameraInterface& getCamera(const Camera camera) = 0;
  /**
   * @brief getFakeData provides access to the fake data of this interface
   * @return a reference to the requested fake data interface
   */
  virtual FakeDataInterface& getFakeData() = 0;
  /**
   * @brief getAudio provides access to the microphones of the robot
   * @return a reference to the audioInterface
   */
  virtual AudioInterface& getAudio() = 0;
  /**
   * @brief getNextCamera returns the next camera
   * @return the current CameraInterface
   */
  virtual CameraInterface& getNextCamera() = 0;
  /**
   * @brief getCurrentCameraType
   * @return the current camera type
   */
  virtual Camera getCurrentCameraType() = 0;
};
