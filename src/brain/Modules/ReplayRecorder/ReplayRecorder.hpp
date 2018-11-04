#pragma once

#include <fstream>

#include "Data/ButtonData.hpp"
#include "Data/CycleInfo.hpp"
#include "Data/FSRSensorData.hpp"
#include "Data/GameControllerState.hpp"
#include "Data/HeadMatrixBuffer.hpp"
#include "Data/IMUSensorData.hpp"
#include "Data/ImageData.hpp"
#include "Data/JointSensorData.hpp"
#include "Data/ReplayData.hpp"
#include "Data/SonarData.hpp"
#include "Data/TeamPlayers.hpp"
#include "Data/WhistleData.hpp"
#include "Framework/Module.hpp"
#include "Tools/Time.hpp"


class Brain;

class ReplayRecorder : public Module<ReplayRecorder, Brain>
{
public:
  /// the name of this module
  ModuleName name = "ReplayRecorder";
  /**
   * @brief ReplayRecorder initializes members
   * @param manager a reference to brain
   */
  ReplayRecorder(const ModuleManagerInterface& manager);
  /// @brief the destructor ompletes the written json file with closing braces
  ~ReplayRecorder();
  /// @brief the modules cycle.
  void cycle();

private:
  /// the minimum time difference between recorded frames
  const Parameter<float> minSecBetweenFrames_;
  /// whether frames should only be recorded while PLAYING
  const Parameter<bool> onlyRecordWhilePlaying_;

  const Dependency<ImageData> imageData_;
  const Dependency<JointSensorData> jointSensorData_;
  const Dependency<IMUSensorData> imuSensorData_;
  const Dependency<FSRSensorData> fsrSensorData_;
  const Dependency<SonarData> sonarData_;
  const Dependency<ButtonData> buttonData_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<HeadMatrixBuffer> headMatrixBuffer_;

  /// the target directory for the records
  const std::string target_;
  /// the target file for the replay data
  const std::string replayJson_;

  /// the filestream for the replay.json file
  std::ofstream frameStream_;
  /// whether the write thread is busy
  std::atomic<bool> writeThreadBusy_;
  /// whether the current frame is the first one
  bool firstFrame_;
  /// the thread which writes the record to disk
  std::thread writeThread_;

  /// the data of the current frame
  ReplayFrame currentFrame_;
  Image currentImage_;
  CVData currentPngImage_;

  /// writes the currentFrame_ to disk
  void writeFrame();
  /// initializes the replay.json with configuration and frame start list
  void initReplay(std::ofstream& fileStream) const;
  /// copies the fsr data from datatype to array
  void copyFsrData(const FSRSensorData::Sensor& sensor,
                   std::array<float, keys::sensor::FSR_MAX>& data) const;
  /// copies the imu data from datatype to array
  void copyImuData(const IMUSensorData& sensor,
                   std::array<float, keys::sensor::IMU_MAX>& data) const;
  /// whether all dependencies are valid in the current cycle
  bool allDependenciesValid() const;
  /// checks and eventually reopens the frameStream_
  void refreshFileStream(std::ofstream& fs) const;
};
