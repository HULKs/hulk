#include "Hardware/Replay/ReplayInterface.hpp"
#include "Data/JointSensorData.hpp"
#include "Hardware/Clock.hpp"
#include "Hardware/Replay/lodepng.h"
#include "Tools/Storage/UniValue/UniValue.h"
#include "Tools/Storage/UniValue/UniValue2Json.hpp"
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <thread>

ReplayInterface::ReplayInterface(std::string path)
  : path_(std::move(path))
{
}

std::optional<Image422> ReplayInterface::loadImage(const std::string& path)
{
  std::vector<unsigned char> image;
  unsigned int width = 0;
  unsigned int height = 0;
  if (lodepng::decode(image, width, height, path) != 0)
  {
    Log<M_TUHHSDK>(LogLevel::WARNING)
        << "Could not read image file. Maybe it does not exist or is not a PNG file? File was "
        << path;
    return {};
  }
  Image422 result{Vector2i(width, height)};
  for (std::size_t y = 0; y < height; y++)
  {
    for (std::size_t x = 0; x < width / 2; x++)
    {
      auto& c = result.at(y, x);
      // Calculate the position in the 444 image:
      // x*2 to restore the 444 coordinate
      // (...)*4 to jump over the y|cb|cb|alpha values
      const auto pos = (y * width + x * 2) * 4;
      c.y1 = image[pos];
      c.y2 = image[pos + 4];
      c.cb = image[pos + 1];
      c.cr = image[pos + 2];
    }
  }
  return result;
}

void ReplayInterface::configure(Configuration& c)
{
  robotInfo_.bodyVersion = RobotVersion::V6;
  robotInfo_.headVersion = RobotVersion::V6;
  robotInfo_.bodyName = "replay";
  robotInfo_.headName = "replay";
  c.setNaoHeadName(robotInfo_.headName);
  c.setNaoBodyName(robotInfo_.bodyName);

  // Export the NaoInfo to provide it in tuhhSDK.base for Export Diff functionality in MATE
  Uni::Value value = Uni::Value(Uni::ValueType::OBJECT);
  value << robotInfo_;
  c.set("tuhhSDK.base", "RobotInfo", value);

  robotMetrics_.configure(c, robotInfo_);

  Json::Reader reader;
  Json::Value root;
  // TODO: Ideally path would be the path to a directory containing FileTransport data
  std::ifstream stream(path_);
  if (!stream.is_open())
  {
    throw std::runtime_error("Could not open file.");
  }
  reader.parse(stream, root);
  Uni::Value replay = Uni::Converter::toUniValue(root);
  if (replay.type() != Uni::ValueType::OBJECT)
  {
    throw std::runtime_error("Root of file is not an object.");
  }
  if (!replay.contains("frames"))
  {
    throw std::runtime_error("The file is valid json but does not contain an array of frames.");
  }
  if (c.get("tuhhSDK.base", "loadReplayConfig").asBool() && replay.contains("config"))
  {
    Uni::Value config = replay["config"];
    config >> fakeData_.replayConfig;
  }
  Uni::Value frames = replay["frames"];
  if (frames.size() == 0u)
  {
    throw std::runtime_error("The file has an empty frames array.");
  }
  frames_.reserve(frames.size());
  images_.reserve(frames.size());
  ReplayFrame replayFrame;
  for (auto it = frames.vectorBegin(); it != frames.vectorEnd(); it++)
  {
    ReplayFrame replayFrame;
    *it >> replayFrame;
    auto imagePath = replayFrame.image;
    if (imagePath.empty())
    {
      continue;
    }
    if (imagePath[0] != '/')
    {
      std::filesystem::path jsonPath(path_);
      imagePath = (jsonPath.parent_path() / imagePath).string();
    }
    const auto image = loadImage(imagePath);
    if (!image.has_value())
    {
      continue;
    }
    frames_.emplace_back(replayFrame);
    images_.emplace_back(image.value());
  }
  frameIter_ = frames_.begin();
  imageIter_ = images_.begin();
}

void ReplayInterface::setJointAngles(const JointsArray<float>& /*unused*/) {}

void ReplayInterface::setJointStiffnesses(const JointsArray<float>& /*unused*/) {}

void ReplayInterface::setLEDs(const Led::Chest& /*chest*/, const Led::Ear& /*leftEar*/,
                              const Led::Ear& /*righEar*/, const Led::Eye& /*leftEye*/,
                              const Led::Eye& /*rightEye*/, const Led::Foot& /*leftFoot*/,
                              const Led::Foot& /*rightFoot*/)
{
}

void ReplayInterface::produceSensorData(CycleInfo& cycleInfo, FSRSensorData& fsrSensorData,
                                        IMUSensorData& imuSensorData,
                                        JointSensorData& jointSensorData, ButtonData& buttonData,
                                        SonarSensorData& sonarSensorData)
{
  fakeData_.currentFrame = *frameIter_;

  jointSensorData.angles = frameIter_->jointAngles;
  // TODO: current, temperature
  jointSensorData.valid = true;

  buttonData.switches = frameIter_->switches;
  bool singlePressDetected{!buttonData.switches.isChestButtonPressed &&
                           chestButtonWasPressedLastCycle_};
  if (singlePressDetected)
  {
    lastChestButtonSinglePress_ = cycleInfo.startTime;
  }
  const auto headButtonsPressed{buttonData.switches.isHeadFrontPressed &&
                                buttonData.switches.isHeadMiddlePressed &&
                                buttonData.switches.isHeadRearPressed};
  if (headButtonsPressed && !headButtonsWerePressedLastCycle_)
  {
    headButtonsPressStart_ = cycleInfo.startTime;
  }
  else if (!headButtonsPressed && headButtonsWerePressedLastCycle_)
  {
    headButtonsPressStart_.reset();
  }
  if (headButtonsPressStart_.has_value() &&
      cycleInfo.getAbsoluteTimeDifference(*headButtonsPressStart_) > 1s)
  {
    lastHeadButtonsHold_ = cycleInfo.startTime;
    headButtonsPressStart_.reset();
  }
  buttonData.lastChestButtonSinglePress = lastChestButtonSinglePress_;
  buttonData.lastHeadButtonsHold = lastHeadButtonsHold_;
  buttonData.valid = true;
  chestButtonWasPressedLastCycle_ = buttonData.switches.isChestButtonPressed;
  headButtonsWerePressedLastCycle_ = headButtonsPressed;

  imuSensorData.accelerometer = frameIter_->imu.accelerometer;
  imuSensorData.gyroscope = frameIter_->imu.gyroscope;
  imuSensorData.angle = frameIter_->imu.angle;
  imuSensorData.valid = true;

  fsrSensorData.leftFoot = frameIter_->fsrLeft;
  fsrSensorData.rightFoot = frameIter_->fsrRight;
  fsrSensorData.totalLeft = fsrSensorData.leftFoot.frontLeft + fsrSensorData.leftFoot.frontRight +
                            fsrSensorData.leftFoot.rearLeft + fsrSensorData.leftFoot.rearRight;
  fsrSensorData.rightFoot = fsrSensorData.rightFoot;
  fsrSensorData.totalRight = fsrSensorData.rightFoot.frontLeft +
                             fsrSensorData.rightFoot.frontRight + fsrSensorData.rightFoot.rearLeft +
                             fsrSensorData.rightFoot.rearRight;
  fsrSensorData.valid = true;

  sonarSensorData.data = frameIter_->sonarDist;
  /// the maximum echo range in meters for the sonar sensors, taken from
  /// http://doc.aldebaran.com/2-1/family/nao_dcm/actuator_sensor_names.html#sonars
  constexpr auto maxSonarRange{5.f};
  // A value <= 0 less means error, >= MAX_DETECTION_RANGE means no echo. Source:
  // http://doc.aldebaran.com/2-1/family/nao_dcm/actuator_sensor_names.html#term-us-sensors-m
  sonarSensorData.valid[Sonars::LEFT] =
      sonarSensorData.data.leftSensor > 0.f && sonarSensorData.data.leftSensor < maxSonarRange;
  sonarSensorData.valid[Sonars::RIGHT] =
      sonarSensorData.data.rightSensor > 0.f && sonarSensorData.data.rightSensor < maxSonarRange;

  cycleInfo.startTime = frameIter_->timestamp;
  cycleInfo.cycleTime = cycleInfo.startTime - lastSensorDataProduction_;
  lastSensorDataProduction_ = cycleInfo.startTime;

  std::this_thread::sleep_for(std::chrono::microseconds(300000));

  rCamera_.setImage(*imageIter_, frameIter_->camera, frameTimestamp_);
  // Since the replayInterface loops the replay images the frameTimestamp read from replay data is
  // not continuously increasing. This increments the timestamp continuously.
  frameTimestamp_ += 10ms;
  // No button callbacks in replay. At least not yet. They could be generated from the switches
  // directly.

  frameIter_++;
  imageIter_++;
  if (frameIter_ == frames_.end())
  {
    frameIter_ = frames_.begin();
    imageIter_ = images_.begin();
  }
}

void ReplayInterface::enableImageDataProducer() {}

void ReplayInterface::disableImageDataProducer() {}

void ReplayInterface::produceImageData(CycleInfo& cycleInfo, ImageData& imageData)
{
  rCamera_.produce(cycleInfo, imageData);

  cycleInfo.cycleTime = cycleInfo.startTime - lastImageDataProduction_;
  lastImageDataProduction_ = cycleInfo.startTime;
}

std::string ReplayInterface::getFileRoot() const
{
  // Replay uses the same file system structure as webots
  return LOCAL_FILE_ROOT;
}

std::string ReplayInterface::getDataRoot() const
{
  return getFileRoot();
}

const RobotInfo& ReplayInterface::getRobotInfo()
{
  return robotInfo_;
}

const RobotMetrics& ReplayInterface::getRobotMetrics()
{
  return robotMetrics_;
}

Clock::time_point ReplayInterface::getRealFrameTime()
{
  return lastSensorDataProduction_;
}

AudioInterface& ReplayInterface::getAudio()
{
  return audioInterface_;
}

FakeDataInterface& ReplayInterface::getFakeData()
{
  return fakeData_;
}
