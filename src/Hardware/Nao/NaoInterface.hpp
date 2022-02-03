#pragma once

#include "Data/ButtonData.hpp"
#include "Data/FSRSensorData.hpp"
#include "Hardware/Nao/NaoAudio.hpp"
#include "Hardware/Nao/NaoCamera.hpp"
#include "Hardware/Nao/NaoFakeData.hpp"
#include "Hardware/Nao/ProxyInterface.hpp"
#include "Hardware/RobotInterface.hpp"
#include <boost/asio.hpp>
#include <fstream>
#include <memory>
#include <mutex>
#include <optional>
#include <thread>
#include <utility>

class NaoInterface : public RobotInterface
{
public:
  NaoInterface();
  NaoInterface(const NaoInterface&) = delete;
  NaoInterface(NaoInterface&&) = delete;
  NaoInterface& operator=(const NaoInterface&) = delete;
  NaoInterface& operator=(NaoInterface&&) = delete;
  ~NaoInterface() override = default;

  void configure(Configuration& config) override;
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
  bool isUSBStickMounted() const;
  const RobotInfo& getRobotInfo() override;
  const RobotMetrics& getRobotMetrics() override;
  AudioInterface& getAudio() override;
  FakeDataInterface& getFakeData() override;

private:
  /**
   * @brief getDataRootAndUSBStickState returns a path where files can be stored during the game,
   * e.g. fileTransport or ReplayRecorder data, and whether the path is located on the USB stick
   * @return pair of path and whether path is located on the USB stick
   */
  std::pair<std::string, bool> getDataRootAndUSBStickState() const;
  /**
   * @brief initRobotInfo converts IDs and version strings to names and enums
   * @param config a reference to the Configuration instance
   */
  void initializeRobotConfiguration(Configuration& config);

  boost::asio::io_context ioContext_{};
  boost::asio::local::stream_protocol::socket socket_;
  boost::asio::local::stream_protocol::endpoint proxyEndpoint_;

  ProxyInterface::StateStorage stateStorage_;
  ProxyInterface::ControlStorage controlStorage_;

  ProxyInterface::RobotConfiguration robotConfiguration_;

  RobotInfo robotInfo_;
  RobotMetrics robotMetrics_;
  NaoFakeData fakeData_;
  NaoAudio audioInterface_;
  NaoCamera topCamera_;
  NaoCamera bottomCamera_;
  CameraPosition currentCamera_;

  Clock::time_point lastSensorDataProduction_;
  Clock::time_point lastImageDataProduction_;

  bool chestButtonWasPressedLastCycle_{false};
  bool headButtonsWerePressedLastCycle_{false};
  Clock::time_point lastChestButtonSinglePress_;
  Clock::time_point lastHeadButtonsHold_;
  std::optional<Clock::time_point> headButtonsPressStart_;
};
