#pragma once

#include "Hardware/Definitions.hpp"
#include "Hardware/RobotInterface.hpp"
#include "Hardware/SimRobot/HULKsMenu.hpp"
#include "Hardware/SimRobot/SimRobotCamera.hpp"
#include "Hardware/SimRobot/SimRobotFakeData.hpp"
#include "Tools/Math/KinematicMatrix.hpp"
#include <SimRobotCore2.h>
#include <chrono>
#include <condition_variable>
#include <memory>
#include <mutex>
#include <string>
#include <tuple>
#include <vector>

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
  SimRobotInterface(SimRobot::Application& application, SimRobot::Object* robot);
  SimRobotInterface(const SimRobotInterface&) = delete;
  SimRobotInterface(SimRobotInterface&&) = delete;
  SimRobotInterface& operator=(const SimRobotInterface&) = delete;
  SimRobotInterface& operator=(SimRobotInterface&&) = delete;
  ~SimRobotInterface() override;

  void update(std::uint64_t simulatedSteps);
  void configure(Configuration& config) override;
  void setJointAngles(const JointsArray<float>& angles) override;
  void setJointStiffnesses(const JointsArray<float>& stiffnesses) override;
  void setLEDs(const Led::Chest& chest, const Led::Ear& leftEar, const Led::Ear& righEar,
               const Led::Eye& leftEye, const Led::Eye& rightEye, const Led::Foot& leftFoot,
               const Led::Foot& rightFoot) override;
  void produceSensorData(CycleInfo& cycleInfo, FSRSensorData& fsrSensorData,
                         IMUSensorData& imuSensorData, JointSensorData& jointSensorData,
                         ButtonData& buttonData, SonarSensorData& sonarSensorData) override;
  void enableImageDataProducer() override;
  void disableImageDataProducer() override;
  void produceImageData(CycleInfo& cycleInfo, ImageData& imageData) override;
  std::string getFileRoot() const override;
  std::string getDataRoot() const override;
  const RobotInfo& getRobotInfo() override;
  const RobotMetrics& getRobotMetrics() override;
  AudioInterface& getAudio() override;
  FakeDataInterface& getFakeData() override;

  void pressChestButton();
  [[nodiscard]] const std::string& getName() const;

private:
  void updateFSRs();
  static KinematicMatrix getKinematicMatrix(SimRobot::Object* object);

  RobotInfo robotInfo_;
  RobotMetrics robotMetrics_;
  SimRobot::Application& application_;
  SimRobot::Object* robot_ = nullptr;
  SimRobot::Object* leftFoot_ = nullptr;
  SimRobot::Object* rightFoot_ = nullptr;
  JointsArray<SimRobot::Object*> jointSensors_{};
  JointsArray<SimRobot::Object*> jointActuators_{};
  SimRobot::Object* gyroscope_ = nullptr;
  SimRobot::Object* accelerometer_ = nullptr;
  std::array<SimRobot::Object*, 2> cameras_{};
  SimRobot::Object* ball_ = nullptr;
  std::vector<SimRobot::Object*> otherRobots_;

  Clock::time_point lastTimePoint_;
  Clock::time_point currentTimePoint_;

  std::mutex jointAnglesMutex_;
  std::condition_variable jointAnglesConditionVariable_;
  bool jointAnglesAvailable_{true};
  JointsArray<float> jointAngles_{};

  SimRobotCamera topCamera_;
  SimRobotCamera bottomCamera_;
  SimRobotFakeData fakeData_;
  std::unique_ptr<AudioInterface> audio_;
  CameraPosition currentCamera_{CameraPosition::TOP};
  std::uint64_t counter_{0};

  std::mutex sensorDataMutex_;
  std::condition_variable sensorDataConditionVariable_;
  bool sensorDataAvailable_{false};
  CycleInfo sensorDataCycleInfo_;
  FSRSensorData sensorDataFSRSensorData_;
  IMUSensorData sensorDataIMUSensorData_;
  JointSensorData sensorDataJointSensorData_;
  ButtonData sensorDataButtonData_;
  SonarSensorData sensorDataSonarSensorData_;
  Clock::time_point lastSensorDataProduction_;

  std::atomic<bool> chestButtonWasRequested_{false};
  bool chestButtonWasPressedLastUpdate_{false};

  std::string robotName_;
  std::atomic<bool> shutdownRequested_{false};

  std::unique_ptr<TUHH> tuhh_;

  std::mutex cameraMutex_;
  std::condition_variable imagesRendered_;
  Clock::time_point lastImageDataProduction_;
};
