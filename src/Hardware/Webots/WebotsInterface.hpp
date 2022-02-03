#pragma once

#include "Hardware/Definitions.hpp"
#include "Hardware/FakeDataInterface.hpp"
#include "Hardware/RobotInterface.hpp"
#include "Hardware/Webots/WebotsAudio.hpp"
#include "Hardware/Webots/WebotsCamera.hpp"
#include "Hardware/Webots/WebotsFakeData.hpp"
#include <atomic>
#include <condition_variable>
#include <mutex>
#include <vector>
#include <webots/Supervisor.hpp>

class WebotsInterface : public RobotInterface, public webots::Supervisor
{
public:
  WebotsInterface();
  WebotsInterface(const WebotsInterface&) = delete;
  WebotsInterface(WebotsInterface&&) = delete;
  WebotsInterface& operator=(const WebotsInterface&) = delete;
  WebotsInterface& operator=(WebotsInterface&&) = delete;
  ~WebotsInterface() override = default;
  void terminate();
  void waitForTermination();
  void configure(Configuration& configuration) override;
  void setJointAngles(const JointsArray<float>& angles) override;
  void setJointStiffnesses(const JointsArray<float>& stiffnesses) override;
  void setLEDs(const Led::Chest& chest, const Led::Ear& leftEar, const Led::Ear& rightEar,
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
  FakeDataInterface& getFakeData() override;
  AudioInterface& getAudio() override;

  static constexpr auto timeStepMilliseconds__{10};

private:
  std::mutex terminationRequestedMutex_;
  std::condition_variable terminationRequestedConditionVariable_;
  bool terminationRequested_{false};

  RobotInfo robotInfo_;
  RobotMetrics robotMetrics_;
  WebotsCamera topCamera_;
  WebotsCamera bottomCamera_;
  WebotsFakeData fakeData_;
  WebotsAudio audio_;

  webots::Accelerometer* accelerometer_{nullptr};
  webots::Gyro* gyroscope_{nullptr};
  webots::InertialUnit* inertialUnit_{nullptr};

  webots::DistanceSensor* distanceSensorLeft_{nullptr};
  webots::DistanceSensor* distanceSensorRight_{nullptr};

  webots::TouchSensor* leftFsrFrontLeft_{nullptr};
  webots::TouchSensor* leftFsrRearLeft_{nullptr};
  webots::TouchSensor* leftFsrFrontRight_{nullptr};
  webots::TouchSensor* leftFsrRearRight_{nullptr};
  webots::TouchSensor* rightFsrFrontLeft_{nullptr};
  webots::TouchSensor* rightFsrRearLeft_{nullptr};
  webots::TouchSensor* rightFsrFrontRight_{nullptr};
  webots::TouchSensor* rightFsrRearRight_{nullptr};

  JointsArray<webots::Motor*> motors_{};
  JointsArray<webots::PositionSensor*> positionSensors_{};
  JointsArray<float> requestedAngles_{};

  webots::Keyboard* keyboard_{nullptr};

  CameraPosition lastRequestedCameraPosition_{CameraPosition::TOP};
  std::atomic<bool> topCameraRequested_{false};
  std::atomic<bool> bottomCameraRequested_{false};

  Clock::time_point lastSensorDataProduction_;
  Clock::time_point lastImageDataProduction_;

  bool chestButtonWasPressedLastCycle_{false};
  Clock::time_point lastChestButtonSinglePress_;
};
