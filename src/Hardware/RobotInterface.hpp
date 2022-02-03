#pragma once

#include "Data/ButtonData.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FSRSensorData.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/ImageData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/SonarData.hpp"
#include "Hardware/AudioInterface.hpp"
#include "Hardware/Clock.hpp"
#include "Hardware/Definitions.hpp"
#include "Hardware/FakeDataInterface.hpp"
#include "Hardware/RobotMetrics.hpp"
#include <array>
#include <chrono>
#include <string>
#include <tuple>
#include <vector>

class Configuration;

enum class RobotVersion
{
  /// some unknown or unsupported version
  UNKNOWN,
  /// version 6 head or body
  V6
};

struct RobotInfo : public Uni::To
{
  /// the version of the body
  RobotVersion bodyVersion;
  /// the version of the head
  RobotVersion headVersion;
  /// a body name, e.g. tuhhnao11
  std::string bodyName;
  /// a head name, e.g. tuhhnao03
  std::string headName;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["bodyVersion"] << static_cast<unsigned int>(bodyVersion);
    value["headVersion"] << static_cast<unsigned int>(headVersion);
    value["bodyName"] << bodyName;
    value["headName"] << headName;
  }
};

class RobotInterface
{
public:
  RobotInterface() = default;
  RobotInterface(const RobotInterface&) = delete;
  RobotInterface(RobotInterface&&) = delete;
  RobotInterface& operator=(const RobotInterface&) = delete;
  RobotInterface& operator=(RobotInterface&&) = delete;
  virtual ~RobotInterface() = default;
  /**
   * @brief configure does things that require configuration files to be loaded for the correct
   * location / NAO
   * @note This method should be called exactly once.
   * @param config a reference to the Configuration instance
   */
  virtual void configure(Configuration& config) = 0;
  /**
   * @brief setJointAngles sets the joint angles for the current cycle
   * @param angles the values of all joint angles
   */
  virtual void setJointAngles(const JointsArray<float>& angles) = 0;
  /**
   * @brief setJointStiffnesses sets the joint stiffnesses for the current cycle
   * @param stiffnesses the values of all joint stiffnesses
   */
  virtual void setJointStiffnesses(const JointsArray<float>& stiffnesses) = 0;
  /**
   * @brief setLEDs sets the LED colors and/or brightnesses
   * @param chest the values of LEDs of the chest
   * @param leftEar the values of LEDs of the left ear
   * @param rightEar the values of LEDs of the right ear
   * @param leftEye the values of LEDs of the left eye
   * @param rightEye the values of LEDs of the right eye
   * @param skull the values of LEDs of the skull
   * @param leftFoot the values of LEDs of the left foot
   * @param rightFoot the values of LEDs of the right foot
   */
  virtual void setLEDs(const Led::Chest& chest, const Led::Ear& leftEar, const Led::Ear& rightEar,
                       const Led::Eye& leftEye, const Led::Eye& rightEye, const Led::Foot& leftFoot,
                       const Led::Foot& rightFoot) = 0;
  virtual void produceSensorData(CycleInfo& cycleInfo, FSRSensorData& fsrSensorData,
                                 IMUSensorData& imuSensorData, JointSensorData& jointSensorData,
                                 ButtonData& buttonData, SonarSensorData& sonarSensorData) = 0;
  virtual void enableImageDataProducer() = 0;
  virtual void disableImageDataProducer() = 0;
  virtual void produceImageData(CycleInfo& cycleInfo, ImageData& imageData) = 0;
  /**
   * @brief getFileRoot returns a path to a directory that contains all files for our program
   * @return a path
   */
  virtual std::string getFileRoot() const = 0;
  /**
   * @brief getDataRoot returns a path where files can be stored during the game, e.g. fileTransport
   * or ReplayRecorder data
   * @return a path
   */
  virtual std::string getDataRoot() const = 0;
  /**
   * @brief getNaoInfo returns the hardware identification
   * @return info, filled with the body/head version and name
   */
  virtual const RobotInfo& getRobotInfo() = 0;
  /**
   * @brief getRobotMetrics returns the robot metrics
   * @return a reference to the robot metrics
   */
  virtual const RobotMetrics& getRobotMetrics() = 0;
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
};
