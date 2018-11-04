#pragma once

#include <chrono>
#include <condition_variable>
#include <memory>
#include <mutex>
#include <string>
#include <vector>

#include <SimRobotCore2.h>

#include "Definitions/keys.h"
#include "Hardware/RobotInterface.hpp"
#include "Tools/Kinematics/KinematicMatrix.h"

#include "SimRobotAudio.hpp"
#include "SimRobotCamera.hpp"
#include "SimRobotFakeData.hpp"


class TUHH;

enum class HeadButtonType
{
  FRONT,
  MIDDLE,
  REAR
};


class SimRobotInterface : public RobotInterface
{
public:
  /**
   * @brief SimRobotInterface constructs members
   * @param application a reference to the SimRobot application
   * @param robot a pointer to the robot object in SimRobot
   */
  SimRobotInterface(SimRobot::Application& application, SimRobot::Object* robot);
  /**
   * @brief ~SimRobotInterface destructs the SimRobotInterface
   */
  ~SimRobotInterface();
  /**
   * @brief update executes the robot control program for one cycle
   */
  void update();
  /**
   * @brief configure is called by the tuhhSDK as soon as all configuration categories are available
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
   * @brief waitAndReadSensorData copies sensor values
   * @param sensors is filled with the sensor data from the last cycle
   */
  void waitAndReadSensorData(NaoSensorData& data);
  /**
   * @brief getFileRoot returns a path to a directory that contains all files for our program
   * @return a path
   */
  std::string getFileRoot();
  /**
   * @brief delegate to getFileRoot
   */
  std::string getDataRoot();
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
   * @brief getAudio provides access to the microphones of the robot
   * @return a reference to the audioInterface
   */
  AudioInterface& getAudio();
  /**
   * @brief getNextCamera returns the next camera
   * @return the next to be processed camera
   */
  CameraInterface& getNextCamera();
  /**
   * @brief getCurrentCameraType
   * @return the current camera type
   */
  Camera getCurrentCameraType();
  /**
   * @brief getFakeData provides access to the fake data of this interface
   * @return a reference to the requested fake data interface
   */
  FakeDataInterface& getFakeData();
  /**
   * @brief pressChestButton simulates a chest button press
   */
  void pressChestButton();
  /**
   * @brief pressHeadButton simulates a head button press
   * @param headButtonType which head button is pressed
   */
  void pressHeadButton(const HeadButtonType headButtonType);
  /**
   * @brief getName returns the name of this robot
   * @return the name of this robot
   */
  const std::string& getName() const;

private:
  /**
   * @brief updateFSRs update the FSR values
   */
  void updateFSRs();

  /**
   * @brief getKinematicMatrix gets the KinematicMatrix of a given body part
   * @param object the object to calculate the KinematicMatrix of
   * @param target out parameter for the KinematicMatrix
   */
  void getKinematicMatrix(SimRobot::Object* object, KinematicMatrix& target) const;

  typedef std::chrono::high_resolution_clock Clock;
  /// the SimRobot application
  SimRobot::Application& application_;
  /// robot object
  SimRobot::Object* robot_ = nullptr;
  /// left foot object
  SimRobot::Object* leftFoot_ = nullptr;
  /// right foot object
  SimRobot::Object* rightFoot_ = nullptr;
  /// objects from which joint angle measurements can be obtained
  SimRobot::Object* jointSensors_[keys::joints::JOINTS_MAX];
  /// objects to which joint commands can be sent
  SimRobot::Object* jointActuators_[keys::joints::JOINTS_MAX];
  /// object from which gyroscope measurements are obtained
  SimRobot::Object* gyroscope_ = nullptr;
  /// object from which accelerometer measurements are obtained
  SimRobot::Object* accelerometer_ = nullptr;
  /// objects from which images can be obtained
  SimRobot::Object* cameras_[2];
  /// objects from which a ball can be obtained
  SimRobot::Object* ball_ = nullptr;
  /// a vector of objects representing all robots in the scene but this one
  std::vector<SimRobot::Object*> otherRobots_;
  /// lock for joint angles that are set from the motion thread
  std::mutex jointAngleLock_;
  /// the last joint angle commands that were set from the motion thread
  std::vector<float> jointAngleCommands_;
  /// a camera providing the top image
  SimRobotCamera topCamera_;
  /// a camera providing the bottom image
  SimRobotCamera bottomCamera_;
  /// the fake data provided by simrobot
  SimRobotFakeData fakeData_;
  /// a dummy audio interface
  SimRobotAudio audio_;
  /// list of callbacks during the last cycle
  std::vector<callback_event> callbacks_;
  /// the current camera
  Camera currentCamera_ = Camera::TOP;
  /// a counter for frames
  unsigned int counter_ = 0;
  /// the array of buttons
  std::array<float, keys::sensor::SWITCH_MAX> buttons_;
  /// the timestamp of the end of the last render cycle
  std::chrono::high_resolution_clock::time_point lastRenderCycleEnd_ = Clock::now();
  /// the name of the robot object in SimRobot
  std::string robotName_;
  /// lock for sensor data as they are accessed from the motion thread
  std::mutex sensorDataLock_;
  /// the next sensor data that will be returned by waitAndReadSensorData
  NaoSensorData sensorData_;
  /// condition variable to wake up motion thread
  std::condition_variable cv_;
  /// condition variable to wake up SimRobot thread
  std::condition_variable jointAngleCv_;
  /// flag that indicates whether new sensor data are present (from SimRobot to motion)
  bool newData_ = false;
  /// flag that indicates whether new joint angles are present (from motion to SimRobot)
  bool newJointAngles_ = true;
  /// whether things should shut down
  bool shutdownRequest_ = false;
  /// the instance of TUHH (should be the last declared member because it should be destroyed before the condition variables)
  std::unique_ptr<TUHH> tuhh_;
  /// lock for camera data as they are accessed from the brain thread
  std::mutex cameraDataLock_;
  /// condition variable to notify the brain thread of newly rendered images
  std::condition_variable imagesRendered_;
};
