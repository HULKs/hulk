#pragma once

#include "Data/HeadMatrixBuffer.hpp"
#include "Data/ReplayData.hpp"
#include "Framework/Configuration/Configuration.h"
#include "Hardware/Replay/ReplayAudio.hpp"
#include "Hardware/Replay/ReplayCamera.hpp"
#include "Hardware/Replay/ReplayFakeData.hpp"
#include "Hardware/RobotInterface.hpp"
#include <array>
#include <optional>

class ReplayInterface : public RobotInterface
{
public:
  /**
   * @brief ReplayInterface reads in a file containing replay frames
   * @param path the path to the file that should be loaded
   */
  explicit ReplayInterface(std::string path);

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

  /**
   * @brief Returns the timestamp the image was generated on the nao
   * @return the timestamp
   */
  Clock::time_point getRealFrameTime();

private:
  /// Replay file path
  std::string path_;
  /**
   * @brief loadImage uses lodepng to read an image from a file
   * @param path the path to the image file
   * @param result the image that is read from the file
   * @return whether the image was successfully loaded
   */
  static std::optional<Image422> loadImage(const std::string& path);

  Clock::time_point frameTimestamp_;
  RobotInfo robotInfo_;
  RobotMetrics robotMetrics_;
  /// stores all the frame data
  std::vector<ReplayFrame> frames_;
  std::vector<Image422> images_;
  /// points to the current frame
  std::vector<ReplayFrame>::const_iterator frameIter_;
  std::vector<Image422>::const_iterator imageIter_;
  /// list of recorded images from the top camera
  ReplayCamera rCamera_;
  /// audio interface dummy
  ReplayAudio audioInterface_;
  /// the fake data provided by replay
  ReplayFakeData fakeData_;
  Clock::time_point lastSensorDataProduction_;

  bool chestButtonWasPressedLastCycle_{false};
  bool headButtonsWerePressedLastCycle_{false};
  Clock::time_point lastChestButtonSinglePress_;
  Clock::time_point lastHeadButtonsHold_;
  std::optional<Clock::time_point> headButtonsPressStart_;

  Clock::time_point lastImageDataProduction_;
};
