#pragma once

#include <array>

#include "Hardware/RobotInterface.hpp"

#include "Definitions/keys.h"

#include "ReplayAudio.hpp"
#include "ReplayCamera.hpp"

struct ReplayFrame
{
  /// joint angle sensor data
  std::array<float, keys::joints::JOINTS_MAX> jointAngles;
  /// measured distance from left sensor
  float sonarLeft;
  /// measured distance from right sensor
  float sonarRight;
  /// data from the left FSR
  std::array<float, keys::sensor::FSR_MAX> fsrLeft;
  /// data from the right FSR
  std::array<float, keys::sensor::FSR_MAX> fsrRight;
  /// data from the IMU (accelerometer, gyroscope, angle)
  std::array<float, keys::sensor::IMU_MAX> imu;
  /// button states
  std::array<float, keys::sensor::SWITCH_MAX> switches;
  /// image for the camera
  Image image;
  /// the timestamp when the frame was recorded
  TimePoint timestamp;
  /// which camera is used
  Camera camera;
};

class ReplayInterface : public RobotInterface
{
public:
  /**
   * @brief ReplayInterface reads in a file containing replay frames
   * @param path the path to the file that should be loaded
   */
  ReplayInterface(const std::string& path);
  /**
   * @brief configure does nothing
   */
  void configure(Configuration&);
  /**
   * @brief setJointAngles sets the joint angles for the current cycle
   * @param angles the values of all joint angles
   */
  void setJointAngles(const std::vector<float>& angles);
  /**
   * @brief setJointStiffnesses sets the joint stiffnesses for the current cycle
   * @param stiffnesses the values of all joint stiffnesses
   */
  void setJointStiffnesses(const std::vector<float>& stiffnesses);
  /**
   * @brief setLEDs sets the LED colors and/or brightnesses
   * @param leds the values of all LEDs
   */
  void setLEDs(const std::vector<float>& leds);
  /**
   * @brief setSonar sets the value of the sonar actuator
   * @param sonar the value of the sonar actuator (see Soft Bank documentation)
   */
  void setSonar(const float sonar);
  /**
   * @brief waitAndReadSensorData waits for some time and copies data from the replay frame
   * @param data is filled with the current replay frame
   */
  void waitAndReadSensorData(NaoSensorData& data);
  /**
   * @brief getFileRoot returns a path to a directory that contains all files for our program
   * @return a path
   */
  std::string getFileRoot();
  /**
   * @brief getNaoInfo copies the hardware identification
   * @param info is filled with the body/head version and name
   */
  void getNaoInfo(Configuration&, NaoInfo& info);
  /**
   * @brief getCamera provides access to the cameras of the robot
   * @param camera an identifier for the camera, i.e. top or bottom camera
   * @return a reference to the requested camera
   */
  CameraInterface& getCamera(const Camera camera);
  /**
   * @brief getCurrentCamera
   */
  CameraInterface& getCurrentCamera();
  /**
   * @brief getAudio provides access to the audio devices of the robot
   * @return a reference to the audio interface
   */
  AudioInterface& getAudio();

private:
  /**
   * @brief loadImage uses lodepng to read an image from a file
   * @param path the path to the image file
   * @param result the image that is read from the file
   */
  void loadImage(const std::string& path, Image& result);
  /// stores all the frame data
  std::vector<ReplayFrame> frames_;
  /// points to the current frame
  std::vector<ReplayFrame>::const_iterator frameIter_;
  /// list of recorded images from the top camera
  ReplayCamera rCamera_;
  /// audio interface dummy
  ReplayAudio audioInterface_;
};
