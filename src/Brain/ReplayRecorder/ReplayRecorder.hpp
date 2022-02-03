#pragma once

#include "Data/BallDetectionReplayRecorderData.hpp"
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
#include "Hardware/Clock.hpp"
#include <filesystem>
#include <fstream>
#include <mutex>
#include <queue>
#include <thread>
#include <utility>


class Brain;

class ReplayRecorder : public Module<ReplayRecorder, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"ReplayRecorder"};

  /**
   * @brief ReplayRecorder initializes members
   * @param manager a reference to brain
   */
  explicit ReplayRecorder(const ModuleManagerInterface& manager);
  /**
   * @brief the destructor completes the written json file with closing braces
   */
  ~ReplayRecorder() override;
  /**
   * @brief the modules cycle.
   */
  void cycle() override;

private:
  struct Frame
  {
    ReplayFrame replayFrame;
    Image422 image;

    Frame(ReplayFrame replayFrame, const Image422& image)
      : replayFrame(std::move(replayFrame))
      , image(image)
    {
    }
  };

  /// the minimum time difference between recorded frames
  const Parameter<Clock::duration> minimumSecondsBetweenFrames_;
  /// whether frames should only be recorded while PLAYING
  const Parameter<bool> onlyRecordWhilePlaying_;
  /// the number of frames to collect at once (number of consecutive cycles to record)
  const Parameter<int> numberOfConsecutiveFrames_;
  /// the maximum number of frames to queue before dropping new frames
  const Parameter<std::size_t> maximumFrameQueueSize_;
  /// whether to record frames from top camera cycle
  const Parameter<bool> disableTopCameraFrames_;
  /// whether to record frames from bottom camera cycle
  const Parameter<bool> disableBottomCameraFrames_;
  /// the minimum available space where the replay recorder stops accepting frame requests from
  /// other modules
  const Parameter<std::uintmax_t> minimumAvailableSpaceStopAcceptingRequests_;
  /// the minimum available space where the replay recorder stops recording at all
  const Parameter<std::uintmax_t> minimumAvailableSpaceStopRecording_;
  /// the minimum time difference between two space checks
  const Parameter<Clock::duration> minimumSecondsBetweenSpaceChecks_;
  /// whether to enable USB stick checks
  const Parameter<bool> enableUSBStickChecks_;
  /// the minimum time difference between two USB stick checks
  const Parameter<Clock::duration> minimumSecondsBetweenUSBStickChecks_;

  const Dependency<ImageData> imageData_;
  const Dependency<JointSensorData> jointSensorData_;
  const Dependency<IMUSensorData> imuSensorData_;
  const Dependency<FSRSensorData> fsrSensorData_;
  const Dependency<SonarSensorData> sonarSensorData_;
  const Dependency<ButtonData> buttonData_;
  const Dependency<CycleInfo> cycleInfo_;
  const Dependency<GameControllerState> gameControllerState_;
  const Dependency<HeadMatrixBuffer> headMatrixBuffer_;
  const Dependency<BallDetectionReplayRecorderData> ballDetectionReplayRecorderData_;

  /// the target directory for the records
  const std::filesystem::path replayDirectory_;
  /// the target file for the replay data
  const std::filesystem::path replayJson_;

  /// the file stream for the replay.json file
  std::ofstream frameStream_;
  /// whether the current frame is the first one
  bool firstFrame_{true};

  /// list of replay frames to write.
  /// ATTENTION: The frames container MUST NOT invalidate existing elements when inserting/deleting
  std::queue<Frame> frames_;
  /// the mutex of the frame queue.
  std::mutex framesMutex_;
  /// the running state of the worker thread. (will be set to false to stop the worker)
  bool framesWorkerRunning_{true};
  /// the condition variable for frame queue mutex and running state.
  std::condition_variable framesConditionVariable_;
  /// the worker thread which writes frames to disk.
  std::thread framesWorker_;

  /// The number of pending frames.
  unsigned int pendingFrames_{};
  /// The time when the last frame was recorded.
  Clock::time_point lastFrameTime_{};

  /// the current space information of the replay directory
  std::filesystem::space_info currentSpace_{0, 0, 0};
  /// the last time when the space was checked.
  Clock::time_point lastSpaceCheck_{};
  /// the last time when warned about missing USB stick.
  Clock::time_point lastUSBStickCheck_{};

  /// writes the queued frames to disk
  void flushQueue();
  /// initializes the replay.json with configuration and frame start list
  void initReplay(std::ofstream& frameStream) const;
  /// whether all dependencies are valid in the current cycle
  bool allDependenciesValid() const;
  /// whether other modules requested a recording
  bool frameRequestedByOthers() const;
  /// checks and eventually reopens the frameStream_
  void refreshFileStream(std::ofstream& fs) const;
  /// worker function which is running in a thread
  void framesWorker();
  static std::filesystem::path getReplayDirectory(const std::filesystem::path& dataRoot);
};
