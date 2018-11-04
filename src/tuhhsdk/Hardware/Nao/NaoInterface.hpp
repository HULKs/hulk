#pragma once

#include <memory>

#include <boost/interprocess/mapped_region.hpp>
#include <boost/interprocess/shared_memory_object.hpp>

#include "Hardware/Nao/SMO.h"
#include "Hardware/RobotInterface.hpp"
#include "NaoAudio.hpp"
#include "NaoCamera.hpp"
#include "NaoFakeData.hpp"

class NaoInterface : public RobotInterface
{
public:
  /**
   * @brief NaoInterface connects to the shared memory of the ALModule
   */
  NaoInterface();
  /**
   * @brief ~NaoInterface
   */
  ~NaoInterface();
  /**
   * @brief configure configures the cameras
   * @param config a reference to the Configuration instance
   */
  void configure(Configuration& config);
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
   * @brief waitAndReadSensorData waits on interprocess condition and copies data from shared memory
   * @param data is filled with data from the shared memory
   */
  void waitAndReadSensorData(NaoSensorData& data);
  /**
   * @brief getFileRoot returns a path to a directory that contains all files for our program
   * @return in this case /home/nao/naoqi/
   */
  std::string getFileRoot();
  /**
   * @brief returns the path to the usb device if present, otherwise $HOME/naoqi
   * @return the path string
   */
  std::string getDataRoot();
  /**
   * @brief getNaoInfo copies the hardware identification
   * @param config a reference to the Configuration instance
   * @param info is filled with the body/head version and name
   */
  void getNaoInfo(Configuration& config, NaoInfo& info);
  /**
   * @brief getCamera provides access to the cameras of the robot
   * @param camera an identifier for the camera, i.e. top or bottom camera
   * @return a reference to the requested camera
   */
  CameraInterface& getCamera(const Camera camera);
  /**
   * @brief getCurrentCameraType
   * @return the current camera type
   */
  Camera getCurrentCameraType();
  /**
   * @brief getNextCamera
   * @return advance to the next to be processed camera
   */
  CameraInterface& getNextCamera();
  /**
   * @brief getAudio provides access to the audio devices of the robot
   * @return a reference to the audio interface
   */
  AudioInterface& getAudio();
  /**
   * @brief getFakeData provides access to the fake data of this interface
   * @return a reference to the requested fake data interface
   */
  FakeDataInterface& getFakeData();

private:
  /**
   * @brief initNaoInfo converts IDs and version strings to names and enums
   * @param config a reference to the Configuration instance
   */
  void initNaoInfo(Configuration& config);
  // Shared memory
  boost::interprocess::shared_memory_object segment_;
  boost::interprocess::mapped_region region_;
  SharedBlock* shmBlock_;
  std::array<char[64], keys::naoinfos::NAOINFO_MAX> rawInfo_;
  NaoInfo naoInfo_;
  NaoCamera topCamera_;
  NaoCamera bottomCamera_;
  NaoAudio audioInterface_;
  NaoFakeData fakeData_;
  Camera currentCamera_;
  uint64_t currentUsedImageTimeStamp;
  uint64_t lastUsedImageTimeStamp;
};
