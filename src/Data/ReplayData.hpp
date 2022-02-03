#pragma once

#include "Data/BallDetectionReplayRecorderData.hpp"
#include "Data/HeadMatrixBuffer.hpp"
#include "Data/MotionOutput.hpp"
#include "Framework/DataType.hpp"
#include "Hardware/Definitions.hpp"
#include "Tools/Storage/Image.hpp"

class ReplayFrame : public DataType<ReplayFrame>
{
public:
  /// the name of this DataType
  DataTypeName name__{"ReplayFrame"};
  /// joint angle sensor data
  JointsArray<float> jointAngles;
  /// measured distance from left and right sonar sensors
  SonarInfo sonarDist;
  /// data from the left FSR
  FSRInfo fsrLeft;
  /// data from the right FSR
  FSRInfo fsrRight;
  /// data from the IMU (accelerometer, gyroscope, angle)
  IMU imu;
  /// button states
  SwitchInfo switches;
  /// image for the camera
  std::string image;
  /// image size in 422 coordinates for the camera
  std::array<int, 2> imageSize422;
  /// which camera is used
  CameraPosition camera;
  /// the timestamp when the frame was recorded
  Clock::time_point timestamp;
  /// the headmatrix buffer which was available in the frame
  HeadMatrixBuffer headMatrixBuffer;
  /// the ball detection data
  BallDetectionData ballDetectionData;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["jointAngles"] << jointAngles;
    value["sonarDist"] << sonarDist;
    value["fsrLeft"] << fsrLeft;
    value["fsrRight"] << fsrRight;
    value["imu"] << imu;
    value["switches"] << switches;
    if (camera == CameraPosition::TOP)
    {
      value["topImage"] << image;
    }
    else
    {
      value["bottomImage"] << image;
    }
    value["imageSize422"] << imageSize422;
    value["timestamp"] << timestamp;
    value["headMatrixBuffer"] << headMatrixBuffer;
    value["ballDetectionData"] << ballDetectionData;
  }

  void reset() override
  {
    jointAngles.fill(0);
    headMatrixBuffer.reset();
    sonarDist = SonarInfo{};
    fsrLeft = FSRInfo{};
    fsrRight = FSRInfo{};
    imu = IMU{};
    switches = SwitchInfo{};
    image.clear();
    imageSize422.fill(0);
    camera = CameraPosition::TOP;
    timestamp = Clock::time_point{};
    ballDetectionData.reset();
  }

  void fromValue(const Uni::Value& value) override
  {
    safeDeserial(
        value, "jointAngles", [](auto& v) { v.fill(0); }, jointAngles);
    safeDeserial(
        value, "headMatrixBuffer", [](auto& v) { v.reset(); }, headMatrixBuffer);
    safeDeserial(
        value, "sonarDist", [](auto& v) { v = SonarInfo{}; }, sonarDist);
    safeDeserial(
        value, "fsrLeft", [](auto& v) { v = FSRInfo{}; }, fsrLeft);
    safeDeserial(
        value, "fsrRight", [](auto& v) { v = FSRInfo{}; }, fsrRight);
    safeDeserial(
        value, "imu", [](auto& v) { v = IMU{}; }, imu);
    safeDeserial(
        value, "switches", [](auto& v) { v = SwitchInfo{}; }, switches);
    if (safeDeserial(
            value, "topImage", [](auto& /*unused*/) {}, image))
    {
      camera = CameraPosition::TOP;
    }
    else if (safeDeserial(
                 value, "bottomImage", [](auto& v) { v = ""; }, image))
    {
      camera = CameraPosition::BOTTOM;
    }
    safeDeserial(
        value, "imageSize422", [](auto& v) { v.fill(0); }, imageSize422);
    // TODO: Find a better default.
    safeDeserial(
        value, "timestamp", [](auto& v) { v = Clock::time_point{}; }, timestamp);
    safeDeserial(
        value, "ballDetectionData", [](auto& v) { v.reset(); }, ballDetectionData);
  }

private:
  template <typename T, typename F>
  inline bool safeDeserial(const Uni::Value& value, const std::string& field, const F fallback,
                           T& target) const
  {
    if (value.contains(field))
    {
      value[field] >> target;
      return true;
    }
    fallback(target);
    return false;
  }
};

struct ReplayConfig : public Uni::From, Uni::To
{
  std::string mount;
  std::string key;
  Uni::Value data;
  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["mount"] << mount;
    value["key"] << key;
    value["data"] << data;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["mount"] >> mount;
    value["key"] >> key;
    value["data"] >> data;
  }
};

class ReplayConfigurations : public DataType<ReplayConfigurations>
{
public:
  /// the name of this DataType
  DataTypeName name__{"ReplayConfigurations"};
  std::vector<ReplayConfig> data;

private:
  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::ARRAY);
    value << data;
  }

  void fromValue(const Uni::Value& value) override
  {
    value >> data;
  }

  void reset() override
  {
    data.clear();
  }
};

class ReplayData : public DataType<ReplayData>
{
public:
  /// the name of this DataType
  DataTypeName name__{"ReplayData"};
  void reset() override {}

  ReplayConfigurations config;
  std::vector<ReplayFrame> frames;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["frames"] << frames;
  }

  void fromValue(const Uni::Value& value) override
  {
    value["frames"] >> frames;
  }
};
