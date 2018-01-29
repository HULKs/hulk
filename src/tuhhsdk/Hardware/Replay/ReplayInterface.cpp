#include <fstream>
#include <iostream>
#include <stdexcept>
#include <thread>

#include "ReplayInterface.hpp"
#include "lodepng.h"

#include "Tools/Storage/UniValue/UniValue.h"
#include "Tools/Storage/UniValue/UniValue2Json.hpp"
#include "Tools/Time.hpp"

ReplayInterface::ReplayInterface(const std::string& path)
{
  Json::Reader reader;
  Json::Value root;
  ReplayFrame replayFrame;
  // TODO: Ideally path would be the path to a directory containing FileTransport data
  std::ifstream stream(path);
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
  Uni::Value frames = replay["frames"];
  if (!frames.size())
  {
    throw std::runtime_error("The file has an empty frames array.");
  }
  frames_.reserve(frames.size());
  for (auto it = frames.listBegin(); it != frames.listEnd(); it++)
  {
    if (it->hasProperty("jointAngles"))
    {
      Uni::Value jointAngles = (*it)["jointAngles"];
      if (jointAngles.size() != keys::joints::JOINTS_MAX)
      {
        throw std::runtime_error("A joint angle array does not have the correct size.");
      }
      jointAngles >> replayFrame.jointAngles;
    }
    else
    {
      replayFrame.jointAngles.fill(0);
    }
    if (it->hasProperty("sonar"))
    {
      Uni::Value sonar = (*it)["sonar"];
      if (sonar.size() != 2)
      {
        throw std::runtime_error("A sonar array does not have the correct size.");
      }
      sonar.at(0) >> replayFrame.sonarLeft;
      sonar.at(1) >> replayFrame.sonarRight;
    }
    else
    {
      replayFrame.sonarLeft = replayFrame.sonarRight = -1.f;
    }
    if (it->hasProperty("fsrLeft"))
    {
      Uni::Value fsrLeft = (*it)["fsrLeft"];
      if (fsrLeft.size() != keys::sensor::FSR_MAX)
      {
        throw std::runtime_error("An fsrLeft array does not have the correct size.");
      }
      fsrLeft >> replayFrame.fsrLeft;
    }
    else
    {
      replayFrame.fsrLeft.fill(0);
    }
    if (it->hasProperty("fsrRight"))
    {
      Uni::Value fsrRight = (*it)["fsrRight"];
      if (fsrRight.size() != keys::sensor::FSR_MAX)
      {
        throw std::runtime_error("An fsrRight array does not have the correct size.");
      }
      fsrRight >> replayFrame.fsrRight;
    }
    else
    {
      replayFrame.fsrRight.fill(0);
    }
    if (it->hasProperty("imu"))
    {
      Uni::Value imu = (*it)["imu"];
      if (imu.size() != keys::sensor::IMU_MAX)
      {
        throw std::runtime_error("An IMU array does not have the correct size.");
      }
      imu >> replayFrame.imu;
    }
    else
    {
      replayFrame.imu.fill(0);
    }
    if (it->hasProperty("switches"))
    {
      Uni::Value switches = (*it)["switches"];
      if (switches.size() != keys::sensor::SWITCH_MAX)
      {
        throw std::runtime_error("A switches array does not have the correct size.");
      }
      switches >> replayFrame.switches;
    }
    else
    {
      replayFrame.switches.fill(0);
    }
    if (it->hasProperty("topImage"))
    {
      Uni::Value topImage = (*it)["topImage"];
      loadImage(topImage.asString(), replayFrame.image);
      replayFrame.camera = Camera::TOP;
    }
    else if (it->hasProperty("bottomImage"))
    {
      Uni::Value bottomImage = (*it)["bottomImage"];
      loadImage(bottomImage.asString(), replayFrame.image);
      replayFrame.camera = Camera::BOTTOM;
    }
    else
    {
      replayFrame.image = Image(Vector2i(1, 1));
    }
    if (it->hasProperty("timestamp"))
    {
      Uni::Value timestamp = (*it)["timestamp"];
      timestamp >> replayFrame.timestamp;
    }
    else
    {
      replayFrame.timestamp = TimePoint::getCurrentTime(); // TODO: Find a better default.
    }
    frames_.push_back(replayFrame);
  }
  frameIter_ = frames_.begin();
}

void ReplayInterface::loadImage(const std::string& path, Image& result)
{
  std::vector<unsigned char> image;
  unsigned int width, height;
  if (lodepng::decode(image, width, height, path.c_str()) != 0)
  {
    throw std::runtime_error("Could not read image file. Maybe it does not exist or is not a PNG file?");
  }
  result = Image(Vector2i(width, height));
  for (unsigned int i = 0; i < (width * height); i++)
  {
    result.data_[i] = Color(image[i * 4], image[i * 4 + 1], image[i * 4 + 2]);
  }
}

void ReplayInterface::configure(Configuration&) {}

void ReplayInterface::setJointAngles(const std::vector<float>&) {}

void ReplayInterface::setJointStiffnesses(const std::vector<float>&) {}

void ReplayInterface::setLEDs(const std::vector<float>&) {}

void ReplayInterface::setSonar(const float) {}

void ReplayInterface::waitAndReadSensorData(NaoSensorData& data)
{
  data.jointSensor = frameIter_->jointAngles;
  // TODO: current, temperature
  data.switches = frameIter_->switches;
  data.imu = frameIter_->imu;
  data.fsrLeft = frameIter_->fsrLeft;
  data.fsrRight = frameIter_->fsrRight;
  data.sonar[keys::sensor::SONAR_LEFT_SENSOR_0] = frameIter_->sonarLeft;
  data.sonar[keys::sensor::SONAR_RIGHT_SENSOR_0] = frameIter_->sonarRight;
  // TODO: battery
  data.time = frameIter_->timestamp;

  std::this_thread::sleep_for(std::chrono::microseconds(300000));

  rCamera_.setImage(frameIter_->image, frameIter_->camera, frameIter_->timestamp);
  // No button callbacks in replay. At least not yet. They could be generated from the switches directly.

  frameIter_++;
  if (frameIter_ == frames_.end())
  {
    frameIter_ = frames_.begin();
  }
}

std::string ReplayInterface::getFileRoot()
{
  // Replay uses the same file system structure as webots
  return LOCAL_FILE_ROOT;
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

CameraInterface& ReplayInterface::getCurrentCamera()
{
  return rCamera_;
}

AudioInterface& ReplayInterface::getAudio()
{
  return audioInterface_;
}
