#pragma once

#include <array>

#include "Data/HeadMatrixBuffer.hpp"
#include "Data/ReplayData.hpp"
#include "Definitions/keys.h"
#include "Hardware/RobotInterface.hpp"
#include "Modules/Configuration/Configuration.h"

#include "ReplayAudio.hpp"
#include "ReplayCamera.hpp"
#include "ReplayFakeData.hpp"

class ReplayInterface : public RobotInterface
{
public:
  /**
   * @brief ReplayInterface reads in a file containing replay frames
   * @param path the path to the file that should be loaded
   */
  ReplayInterface(const std::string& path);

  void configure(Configuration&, NaoInfo&) override;
  void setJointAngles(const std::vector<float>& angles) override;
  void setJointStiffnesses(const std::vector<float>& stiffnesses) override;
  void setLEDs(const std::vector<float>& leds) override;
  void setSonar(const float sonar) override;
  float waitAndReadSensorData(NaoSensorData& data) override;
  std::string getFileRoot() override;
  std::string getDataRoot() override;
  void getNaoInfo(Configuration& config, NaoInfo& info) override;
  CameraInterface& getCamera(const Camera camera) override;
  AudioInterface& getAudio() override;
  CameraInterface& getNextCamera() override;
  Camera getCurrentCameraType() override;
  FakeDataInterface& getFakeData() override;

  /**
   * @brief Returns the timestamp the image was generated on the nao
   * @return the timestamp
   */
  TimePoint getRealFrameTime();

private:
  /// Replay file path
  std::string path_;
  /**
   * @brief loadImage uses lodepng to read an image from a file
   * @param path the path to the image file
   * @param result the image that is read from the file
   * @return whether the image was successfully loaded
   */
  bool loadImage(const std::string& path, Image422& result);

  TimePoint frameTimestamp_;
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
  /// the time point this frame was captured on the nao (nao system time)
  TimePoint realFrameTime_;
};
