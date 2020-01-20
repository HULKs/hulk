#include <boost/filesystem.hpp>

#include "ReplayRecorder.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Time.hpp"
#include "print.h"

#include <Modules/Debug/PngConverter.h>
#include <Modules/NaoProvider.h>
#include <Tools/Storage/UniValue/UniValue2JsonString.h>
#include <thread>


ReplayRecorder::ReplayRecorder(const ModuleManagerInterface& manager)
  : Module(manager)

  , minSecBetweenFrames_(*this, "minSecBetweenFrames", [] {})
  , onlyRecordWhilePlaying_(*this, "onlyRecordWhilePlaying", [] {})
  , numberOfConsecutiveFrames_(*this, "numberOfConsecutiveFrames", [] {})
  , disableTopCameraFrames_(*this, "disableTopCameraFrames", [] {})
  , disableBottomCameraFrames_(*this, "disableBottomCameraFrames", [] {})

  , imageData_(*this)
  , jointSensorData_(*this)
  , imuSensorData_(*this)
  , fsrSensorData_(*this)
  , sonarData_(*this)
  , buttonData_(*this)
  , cycleInfo_(*this)
  , gameControllerState_(*this)
  , headMatrixBuffer_(*this)

  , target_(robotInterface().getDataRoot() + "replay_" +
            std::to_string(TimePoint::getBaseTime() + TimePoint::getCurrentTime().getSystemTime()))
  , replayJson_(target_ + "/replay.json")
  , writeThreadBusy_(false)
  , firstFrame_(true)
  , lastFrameTime_()
{
  images_.reserve(static_cast<unsigned long>(numberOfConsecutiveFrames_()));
  replayFrames_.reserve(static_cast<unsigned long>(numberOfConsecutiveFrames_()));
}

void ReplayRecorder::refreshFileStream(std::ofstream& fs) const
{
  if (!fs.is_open())
  {
    boost::filesystem::create_directory(target_);
    fs.open(replayJson_, std::ios_base::out | std::ios_base::trunc);
  }
}

void ReplayRecorder::initReplay(std::ofstream& frameStream) const
{
  auto configMounts = configuration().getMountPoints();
  std::vector<ReplayConfig> configs;
  for (auto& entry : configMounts)
  {
    auto& mount = entry.first;
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

void ReplayRecorder::writeFrame()
{
  PngConverter imageConverter;
  std::ofstream imageStream;
  for (uint16_t i = 0; i < replayFrames_.size(); i++)
  {
    Uni::Value frame;
    frame << replayFrames_[i];
    const std::string frameString = Uni::Converter::toJsonString(frame, false);

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

    imageConverter.convert(images_[i], currentPngImage_);
    imageStream.open(target_ + "/" + replayFrames_[i].image,
        std::ios_base::out | std::ios_base::trunc | std::ios_base::binary);
    imageStream.write(reinterpret_cast<const char*>(currentPngImage_.data()),
        currentPngImage_.size());
    imageStream.close();
  }

  replayFrames_.clear();
  images_.clear();

  writeThreadBusy_ = false;
}

bool ReplayRecorder::allDependenciesValid() const
{
  return imageData_->is_provided && jointSensorData_->valid && imuSensorData_->valid &&
         fsrSensorData_->valid && buttonData_->valid && cycleInfo_->valid &&
         gameControllerState_->valid && headMatrixBuffer_->valid;
}

void ReplayRecorder::cycle()
{
  // only record when timeDiff to last burst (or single frame) is high enough.
  bool firstFrameInBurst = replayFrames_.empty();
  if (firstFrameInBurst && std::abs(getTimeDiff(cycleInfo_->startTime, lastFrameTime_, TDT::SECS)) <
                               minSecBetweenFrames_())
  {
    return;
  }
  // Only record if the data is available
  if (!allDependenciesValid())
  {
    Log(LogLevel::DEBUG) << "Replay Recorder: Dependency invalid, skipping a cycle";
    return;
  }
  // Only record when unpenalized
  if (gameControllerState_->penalty != Penalty::NONE)
  {
    return;
  }
  // Only record while playing if configured
  if (onlyRecordWhilePlaying_() && gameControllerState_->gameState != GameState::PLAYING)
  {
    return;
  }
  // Only record when currently not busy
  if (writeThreadBusy_)
  {
    return;
  }
  // Check if we want to record this frame according to the camera tye
  if ((disableTopCameraFrames_() && imageData_->camera == Camera::TOP) ||
      (disableBottomCameraFrames_() && imageData_->camera == Camera::BOTTOM))
  {
    return;
  }
  const bool bothCamerasEnabled = !(disableTopCameraFrames_() || disableBottomCameraFrames_());
  // Only record when camera is different to last one (only relevant when both cameras are enabled
  // as we can except that the camera identification does not change when only recording top or
  // bottom images)
  // This ensures that we capture the same amount of top and bottom frames.
  if (bothCamerasEnabled && currentFrame_.camera == imageData_->camera)
  {
    Log(LogLevel::INFO) << "Image not updated, skipping a cycle";
    return;
  }

  // Cycle time is only interesting when recording, therefore the new scope
  {
    Chronometer time(debug(), mount_ + ".cycle_time");

    const std::string imageTarget = imageData_->identification + "Image_" +
                                    std::to_string(imageData_->timestamp.getSystemTime()) + ".png";

    // Copy the image
    images_.emplace_back(imageData_->image422.to444Image());

    // Copy the sensor data
    currentFrame_.jointAngles = jointSensorData_->angles;
    currentFrame_.sonarDist.at(SONARS::LEFT) = sonarData_->filteredValues.at(SONARS::LEFT);
    currentFrame_.sonarDist.at(SONARS::RIGHT) = sonarData_->filteredValues.at(SONARS::RIGHT);
    currentFrame_.sonarValid.at(SONARS::LEFT) = sonarData_->valid.at(SONARS::LEFT);
    currentFrame_.sonarValid.at(SONARS::RIGHT) = sonarData_->valid.at(SONARS::RIGHT);
    copyFsrData(fsrSensorData_->left, currentFrame_.fsrLeft);
    copyFsrData(fsrSensorData_->right, currentFrame_.fsrRight);
    copyImuData(*imuSensorData_, currentFrame_.imu);
    currentFrame_.switches = buttonData_->buttons;
    currentFrame_.image = imageTarget;
    currentFrame_.camera = imageData_->camera;
    currentFrame_.timestamp = imageData_->timestamp;
    currentFrame_.headMatrixBuffer = (*headMatrixBuffer_);

    replayFrames_.push_back(currentFrame_);

    lastFrameTime_ = cycleInfo_->startTime;

    // wait for more frames.
    if (replayFrames_.size() < static_cast<unsigned long>(numberOfConsecutiveFrames_()))
    {
      return;
    }

    // Set write thread busy
    writeThreadBusy_ = true;

    // Start write thread
    if (writeThread_.joinable())
    {
      writeThread_.join();
    }
    writeThread_ =
        std::thread(
            [this] {
              try
              {
                writeFrame();
              }
              catch (...)
              {
                Log(LogLevel::ERROR)
                    << "Unable to write replay frame to disk. Replay file might be broken!";
              }
            });

#ifdef NAO
    // Set priority of write thread very low
    sched_param sch{};
    int policy;
    pthread_getschedparam(writeThread_.native_handle(), &policy, &sch);
    sch.sched_priority = 99;
#endif

    // Detach thread
    writeThread_.detach();
  }
}

void ReplayRecorder::copyFsrData(const FSRSensorData::Sensor& sensor,
                                 std::array<float, keys::sensor::FSR_MAX>& data) const
{
  data[keys::sensor::fsr::FSR_FRONT_LEFT] = sensor.frontLeft;
  data[keys::sensor::fsr::FSR_FRONT_RIGHT] = sensor.frontRight;
  data[keys::sensor::fsr::FSR_REAR_LEFT] = sensor.rearLeft;
  data[keys::sensor::fsr::FSR_REAR_RIGHT] = sensor.rearRight;
  data[keys::sensor::fsr::FSR_TOTAL_WEIGHT] = sensor.totalWeight;
  data[keys::sensor::fsr::FSR_COP_X] = sensor.cop.x();
  data[keys::sensor::fsr::FSR_COP_Y] = sensor.cop.y();
}

void ReplayRecorder::copyImuData(const IMUSensorData& sensor,
                                 std::array<float, keys::sensor::IMU_MAX>& data) const
{
  data[keys::sensor::imu::IMU_ACC_X] = sensor.accelerometer.x();
  data[keys::sensor::imu::IMU_ACC_Y] = sensor.accelerometer.y();
  data[keys::sensor::imu::IMU_ACC_Z] = sensor.accelerometer.z();
  data[keys::sensor::imu::IMU_ANGLE_X] = sensor.angle.x();
  data[keys::sensor::imu::IMU_ANGLE_Y] = sensor.angle.y();
  data[keys::sensor::imu::IMU_ANGLE_Z] = sensor.angle.z();
  data[keys::sensor::imu::IMU_GYR_X] = sensor.gyroscope.x();
  data[keys::sensor::imu::IMU_GYR_Y] = sensor.gyroscope.y();
  data[keys::sensor::imu::IMU_GYR_Z] = sensor.gyroscope.z();
}

ReplayRecorder::~ReplayRecorder()
{
  // wait for probably busy write thread
  if (writeThread_.joinable())
  {
    writeThread_.join();
  }
  refreshFileStream(frameStream_);
  frameStream_ << "]}" << std::endl;
  frameStream_.close();
}
