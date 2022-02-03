#include "Brain/ReplayRecorder/ReplayRecorder.hpp"
#include "Framework/Log/Log.hpp"
#include "Hardware/Clock.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Storage/Image422.hpp"
#include "Tools/Storage/UniValue/UniValue2JsonString.h"
#include <chrono>
#include <filesystem>
#include <ios>
#include <thread>

#ifdef HULK_TARGET_NAO
#include "Hardware/Nao/NaoInterface.hpp"
#endif

ReplayRecorder::ReplayRecorder(const ModuleManagerInterface& manager)
  : Module(manager)
  , minimumSecondsBetweenFrames_{*this, "minimumSecondsBetweenFrames", [] {}}
  , onlyRecordWhilePlaying_{*this, "onlyRecordWhilePlaying", [] {}}
  , numberOfConsecutiveFrames_{*this, "numberOfConsecutiveFrames", [] {}}
  , maximumFrameQueueSize_{*this, "maximumFrameQueueSize", [] {}}
  , disableTopCameraFrames_{*this, "disableTopCameraFrames", [] {}}
  , disableBottomCameraFrames_{*this, "disableBottomCameraFrames", [] {}}
  , minimumAvailableSpaceStopAcceptingRequests_{*this, "minimumAvailableSpaceStopAcceptingRequests",
                                                [] {}}
  , minimumAvailableSpaceStopRecording_{*this, "minimumAvailableSpaceStopRecording", [] {}}
  , minimumSecondsBetweenSpaceChecks_{*this, "minimumSecondsBetweenSpaceChecks", [] {}}
  , enableUSBStickChecks_{*this, "enableUSBStickChecks", [] {}}
  , minimumSecondsBetweenUSBStickChecks_{*this, "minimumSecondsBetweenUSBStickChecks", [] {}}
  , imageData_{*this}
  , jointSensorData_{*this}
  , imuSensorData_{*this}
  , fsrSensorData_{*this}
  , sonarSensorData_{*this}
  , buttonData_{*this}
  , cycleInfo_{*this}
  , gameControllerState_{*this}
  , headMatrixBuffer_{*this}
  , ballDetectionReplayRecorderData_{*this}
  , replayDirectory_{getReplayDirectory(robotInterface().getDataRoot())}
  , replayJson_{replayDirectory_ / "replay.json"}
  , framesWorker_{&ReplayRecorder::framesWorker, this}
{
}

void ReplayRecorder::refreshFileStream(std::ofstream& fs) const
{
  if (!fs.is_open())
  {
    std::filesystem::create_directory(replayDirectory_);
    fs.open(replayJson_, std::ios_base::out | std::ios_base::trunc);
  }
}

void ReplayRecorder::initReplay(std::ofstream& frameStream) const
{
  auto configMounts = configuration().getMountPoints();
  std::vector<ReplayConfig> configs;
  for (auto& entry : configMounts)
  {
    const auto& mount{entry.first};
    for (auto& key : configuration().getKeyList(mount))
    {
      auto& configData = configuration().get(mount, key);
      ReplayConfig config;
      config.mount = mount;
      config.key = key;
      config.data = configData;
      configs.push_back(config);
    }
  }
  Uni::Value exportConfig;
  exportConfig << configs;
  const std::string configString = Uni::Converter::toJsonString(exportConfig, false);
  frameStream << "{ \"config\":" << configString << "," << std::endl;
  frameStream << "\"frames\": [" << std::endl;
}

void ReplayRecorder::flushQueue()
{
  std::ofstream imageStream;
  imageStream.exceptions(std::ofstream::failbit | std::ofstream::badbit);
  std::unique_lock<std::mutex> lock{framesMutex_};
  while (!frames_.empty())
  {
    auto& frame = frames_.front();
    lock.unlock();

    Uni::Value frameValue;
    frameValue << frame.replayFrame;
    const std::string frameString = Uni::Converter::toJsonString(frameValue, false);

    refreshFileStream(frameStream_);
    if (firstFrame_)
    {
      initReplay(frameStream_);
      firstFrame_ = false;
    }
    else
    {
      frameStream_ << ",";
    }
    frameStream_ << frameString << std::endl;

    imageStream.open(replayDirectory_ / frame.replayFrame.image,
                     std::ios_base::out | std::ios_base::trunc | std::ios_base::binary);
    imageStream.write(reinterpret_cast<const char*>(frame.image.data),
                      frame.image.size.x() * frame.image.size.y() *
                          static_cast<std::streamsize>(sizeof(YCbCr422)));
    imageStream.close();

    lock.lock();
    frames_.pop();
  }
}

bool ReplayRecorder::allDependenciesValid() const
{
  return imageData_->valid && jointSensorData_->valid && imuSensorData_->valid &&
         fsrSensorData_->valid && buttonData_->valid && cycleInfo_->valid &&
         gameControllerState_->valid && headMatrixBuffer_->valid;
}

bool ReplayRecorder::frameRequestedByOthers() const
{
  return ballDetectionReplayRecorderData_->recordingRequested;
}

// NOLINTNEXTLINE(readability-function-cognitive-complexity)
void ReplayRecorder::cycle()
{
#ifdef HULK_TARGET_NAO
  if (enableUSBStickChecks_() && (lastUSBStickCheck_ == Clock::time_point{} ||
                                  cycleInfo_->getAbsoluteTimeDifference(lastUSBStickCheck_) >=
                                      minimumSecondsBetweenUSBStickChecks_()))
  {
    lastUSBStickCheck_ = cycleInfo_->startTime;
    if (const auto naoInterface{dynamic_cast<const NaoInterface*>(&robotInterface())};
        naoInterface != nullptr && !naoInterface->isUSBStickMounted())
    {
      debug().playAudio("USB stick missing", AudioSounds::USB_STICK_MISSING);
      Log<M_BRAIN>(LogLevel::WARNING)
          << "Attention: USB Stick is missing. Please insert one and then reboot.";
    }
  }
#endif

  // Update current space informations
  if (lastSpaceCheck_ == Clock::time_point{} ||
      cycleInfo_->getAbsoluteTimeDifference(lastSpaceCheck_) >= minimumSecondsBetweenSpaceChecks_())
  {
    refreshFileStream(frameStream_);
    currentSpace_ = std::filesystem::space(replayDirectory_);
    lastSpaceCheck_ = cycleInfo_->startTime;
  }

  // Only record if the data is available
  if (!allDependenciesValid())
  {
    Log<M_BRAIN>(LogLevel::WARNING) << "Replay Recorder: Dependency invalid, skipping a cycle";
    return;
  }

  if (frameRequestedByOthers())
  {
    pendingFrames_ +=
        currentSpace_.available > minimumAvailableSpaceStopAcceptingRequests_() ? 1 : 0;
  }

  // Only record when unpenalized (overwrite when frame is requested)
  if (!frameRequestedByOthers() && gameControllerState_->penalty != Penalty::NONE)
  {
    return;
  }

  // Only record while playing if configured (overwrite when frame is requested)
  if (!frameRequestedByOthers() && onlyRecordWhilePlaying_() &&
      gameControllerState_->gameState != GameState::PLAYING)
  {
    return;
  }

  // Check if we want to record this frame according to the camera tye (overwrite when frame is
  // requested)
  if (!frameRequestedByOthers() &&
      ((disableTopCameraFrames_() && imageData_->cameraPosition == CameraPosition::TOP) ||
       (disableBottomCameraFrames_() && imageData_->cameraPosition == CameraPosition::BOTTOM)))
  {
    return;
  }

  // Update number of pending frames when there are no pending frames and timeDiff to last burst (or
  // single frame) is high enough.
  if (pendingFrames_ == 0 && currentSpace_.available > minimumAvailableSpaceStopRecording_() &&
      cycleInfo_->getAbsoluteTimeDifference(lastFrameTime_) >= minimumSecondsBetweenFrames_())
  {
    pendingFrames_ += numberOfConsecutiveFrames_();
  }

  // Decrement number of pending frames and check if this cycle should be recorded.
  if (pendingFrames_ > 0)
  {
    pendingFrames_--;
  }
  else
  {
    return;
  }

  // Skip this frame if queue is full
  {
    std::lock_guard<std::mutex> lock{framesMutex_};
    if (frames_.size() >= maximumFrameQueueSize_())
    {
      Log<M_BRAIN>(LogLevel::WARNING) << "Replay Recorder: Frame queue is full, skipping a cycle";
      return;
    }
  }

  // Cycle time is only interesting when recording, therefore the new scope
  {
    Chronometer time(debug(), mount_ + ".cycle_time");

    const std::string imageTarget{
        imageData_->identification + "Image_" +
        std::to_string(std::chrono::duration_cast<std::chrono::milliseconds>(
                           imageData_->captureTimePoint.time_since_epoch())
                           .count()) +
        ".422"};

    ReplayFrame replayFrame;
    replayFrame.jointAngles = jointSensorData_->angles;
    replayFrame.sonarDist = sonarSensorData_->data;
    replayFrame.fsrLeft = fsrSensorData_->leftFoot;
    replayFrame.fsrRight = fsrSensorData_->rightFoot;
    replayFrame.imu.gyroscope = imuSensorData_->gyroscope;
    replayFrame.imu.angle = imuSensorData_->angle;
    replayFrame.imu.accelerometer = imuSensorData_->accelerometer;
    replayFrame.switches = buttonData_->switches;
    replayFrame.image = imageTarget;
    replayFrame.imageSize422 = {imageData_->image422.size.x(), imageData_->image422.size.y()};
    replayFrame.camera = imageData_->cameraPosition;
    replayFrame.timestamp = imageData_->captureTimePoint;
    replayFrame.headMatrixBuffer = (*headMatrixBuffer_);
    replayFrame.ballDetectionData = ballDetectionReplayRecorderData_->data;

    {
      std::lock_guard<std::mutex> lock{framesMutex_};
      frames_.emplace(replayFrame, imageData_->image422);
    }
    framesConditionVariable_.notify_one();

    lastFrameTime_ = cycleInfo_->startTime;
  }
}

ReplayRecorder::~ReplayRecorder()
{
  {
    std::lock_guard<std::mutex> lock{framesMutex_};
    framesWorkerRunning_ = false;
  }
  framesConditionVariable_.notify_one();
  if (framesWorker_.joinable())
  {
    framesWorker_.join();
  }
  try
  {
    flushQueue();
    refreshFileStream(frameStream_);
    frameStream_ << "]}" << std::endl;
    frameStream_.close();
  }
  catch (const std::exception& e)
  {
    Log<M_BRAIN>(LogLevel::ERROR) << "Exception in ReplayRecorder::~ReplayRecorder(): " << e.what()
                                  << ", worker thread terminated.";
  }
  catch (...)
  {
    Log<M_BRAIN>(LogLevel::ERROR)
        << "Unknown exception in ReplayRecorder::~ReplayRecorder(), worker thread terminated.";
  }
}

void ReplayRecorder::framesWorker()
{
  try
  {
    frameStream_.exceptions(std::ofstream::failbit | std::ofstream::badbit);
    std::unique_lock<std::mutex> lock{framesMutex_};
    while (framesWorkerRunning_)
    {
      lock.unlock();

      flushQueue();

      lock.lock();
      framesConditionVariable_.wait(lock,
                                    [this]() { return !framesWorkerRunning_ || !frames_.empty(); });
    }
  }
  catch (const std::exception& e)
  {
    Log<M_BRAIN>(LogLevel::ERROR) << "Exception in ReplyRecorder::framesWorker(): " << e.what()
                                  << ", worker thread terminated.";
  }
  catch (...)
  {
    Log<M_BRAIN>(LogLevel::ERROR)
        << "Unknown exception in ReplayRecorder::framesWorker(), worker thread terminated.";
  }
}

std::filesystem::path ReplayRecorder::getReplayDirectory(const std::filesystem::path& dataRoot)
{
  auto replayDirectory{dataRoot / "replay_0"};
  std::uint32_t replayDirectoryId{0};
  while (std::filesystem::is_directory(replayDirectory))
  {
    ++replayDirectoryId;
    replayDirectory = dataRoot / ("replay_" + std::to_string(replayDirectoryId));
  }
  return replayDirectory;
}
