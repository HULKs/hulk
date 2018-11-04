#include <fstream>
#include <iostream>
#include <stdexcept>
#include <thread>

#include "ReplayInterface.hpp"
#include "lodepng.h"

#include "Tools/Storage/UniValue/UniValue.h"
#include "Tools/Storage/UniValue/UniValue2Json.hpp"
#include "Tools/Time.hpp"
#include <boost/filesystem.hpp>

ReplayInterface::ReplayInterface(const std::string& path)
  : path_(path)
{
}

bool ReplayInterface::loadImage(const std::string& path, Image422& result)
{
  std::vector<unsigned char> image;
  unsigned int width, height;
  if (lodepng::decode(image, width, height, path.c_str()) != 0)
  {
    Log(LogLevel::WARNING)
        << "Could not read image file. Maybe it does not exist or is not a PNG file? File was "
        << path;
    return false;
  }
  result = Image422(Vector2i(width, height));
  for (unsigned int y = 0; y < height; y++)
  {
    for (unsigned int x = 0; x < width / 2; x++)
    {
      auto& c = result.at(y, x);
      // Calculate the position in the 444 image:
      // x*2 to restore the 444 coordinate
      // (...)*4 to jump over the y|cb|cb|alpha values
      const int pos = (y * width + x * 2) * 4;
      c.y1_ = image[pos];
      c.y2_ = image[pos + 4];
      c.cb_ = image[pos + 1];
      c.cr_ = image[pos + 2];
    }
  }
  return true;
}

void ReplayInterface::configure(Configuration& c)
{
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
  if (!replay.hasProperty("frames"))
  {
    throw std::runtime_error("The file is valid json but does not contain an array of frames.");
  }
  if (c.get("tuhhSDK.base", "loadReplayConfig").asBool() && replay.hasProperty("config"))
  {
    Uni::Value config = replay["config"];
    config >> fakeData_.replayConfig;
  }
  Uni::Value frames = replay["frames"];
  if (!frames.size())
  {
    throw std::runtime_error("The file has an empty frames array.");
  }
  frames_.reserve(frames.size());
  images_.reserve(frames.size());
  ReplayFrame replayFrame;
  Image422 image;
  for (auto it = frames.listBegin(); it != frames.listEnd(); it++)
  {
    ReplayFrame replayFrame;
    *it >> replayFrame;
    auto imagePath = replayFrame.image;
    if (imagePath == "")
    {
      continue;
    }
    if (imagePath[0] != '/')
    {
      boost::filesystem::path jsonPath(path_);
      imagePath = (jsonPath.parent_path() / imagePath).string();
    }
    if (!loadImage(imagePath, image))
    {
      continue;
    }
    frames_.push_back(replayFrame);
    images_.push_back(image);
  }
  frameIter_ = frames_.begin();
  imageIter_ = images_.begin();
}

void ReplayInterface::setJointAngles(const std::vector<float>&) {}

void ReplayInterface::setJointStiffnesses(const std::vector<float>&) {}

void ReplayInterface::setLEDs(const std::vector<float>&) {}

void ReplayInterface::setSonar(const float) {}

void ReplayInterface::waitAndReadSensorData(NaoSensorData& data)
{
  fakeData_.currentFrame = *frameIter_;

  data.jointSensor = frameIter_->jointAngles;
  // TODO: current, temperature
  data.switches = frameIter_->switches;
  data.imu = frameIter_->imu;
  data.fsrLeft = frameIter_->fsrLeft;
  data.fsrRight = frameIter_->fsrRight;
  data.sonar[keys::sensor::SONAR_LEFT_SENSOR_0] = frameIter_->sonarDist[0];
  data.sonar[keys::sensor::SONAR_RIGHT_SENSOR_0] = frameIter_->sonarDist[1];
  // TODO: battery
  data.time = frameIter_->timestamp;

  std::this_thread::sleep_for(std::chrono::microseconds(300000));

  rCamera_.setImage(*imageIter_, frameIter_->camera, frameIter_->timestamp);
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

std::string ReplayInterface::getFileRoot()
{
  // Replay uses the same file system structure as webots
  return LOCAL_FILE_ROOT;
}

std::string ReplayInterface::getDataRoot()
{
  return getFileRoot();
}

void ReplayInterface::getNaoInfo(Configuration&, NaoInfo& info)
{
  info.bodyVersion = NaoVersion::V3_3;
  info.headVersion = NaoVersion::V4;
  info.bodyName = "webots";
  info.headName = "webots";
}

CameraInterface& ReplayInterface::getCamera(const Camera)
{
  return rCamera_;
}

CameraInterface& ReplayInterface::getNextCamera()
{
  return rCamera_;
}

Camera ReplayInterface::getCurrentCameraType()
{
  return rCamera_.getCameraType();
}

AudioInterface& ReplayInterface::getAudio()
{
  return audioInterface_;
}

FakeDataInterface& ReplayInterface::getFakeData()
{
  return fakeData_;
}
