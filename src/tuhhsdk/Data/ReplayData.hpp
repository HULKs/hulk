#pragma once

#include "Data/HeadMatrixBuffer.hpp"
#include "Definitions/keys.h"
#include "Framework/DataType.hpp"
#include "Hardware/CameraInterface.hpp"
#include "Tools/Storage/Image.hpp"
#include <Data/MotionOutput.hpp>

class ReplayFrame : public DataType<ReplayFrame>
{
public:
  /// the name of this DataType
  DataTypeName name = "ReplayFrame";
  /// joint angle sensor data
  std::array<float, keys::joints::JOINTS_MAX> jointAngles;
  /// measured distance from left and right sonar sensors
  std::array<float, 2> sonarDist;
  /// data validity for left and right sonar sensors
  std::array<bool, 2> sonarValid;
  /// data from the left FSR
  std::array<float, keys::sensor::FSR_MAX> fsrLeft;
  /// data from the right FSR
  std::array<float, keys::sensor::FSR_MAX> fsrRight;
  /// data from the IMU (accelerometer, gyroscope, angle)
  std::array<float, keys::sensor::IMU_MAX> imu;
  /// button states
  std::array<float, keys::sensor::SWITCH_MAX> switches;
  /// image for the camera
  std::string image;
  /// which camera is used
  Camera camera;
  /// the timestamp when the frame was recorded
  TimePoint timestamp;
  /// the headmatrix buffer which was available in the frame
  HeadMatrixBuffer headMatrixBuffer;

  void toValue(Uni::Value& value) const override
  {
    value = Uni::Value(Uni::ValueType::OBJECT);
    value["jointAngles"] << jointAngles;
    value["sonarDist"] << sonarDist;
    value["sonarValid"] << sonarValid;
    value["fsrLeft"] << fsrLeft;
    value["fsrRight"] << fsrRight;
    value["imu"] << imu;
    value["switches"] << switches;
    if (camera == Camera::TOP)
    {
      value["topImage"] << image;
    }
    else
    {
      value["bottomImage"] << image;
    }
    value["timestamp"] << timestamp;
    value["headMatrixBuffer"] << headMatrixBuffer;
  }

  void reset() override
  {
    jointAngles.fill(0);
    headMatrixBuffer.reset();
    sonarDist.fill(-1.f);
    sonarValid.fill(false);
    fsrLeft.fill(0);
    fsrRight.fill(0);
    imu.fill(0);
    switches.fill(0);
    image = "";
    camera = Camera::TOP;
    timestamp = TimePoint::getCurrentTime();
  }

  void fromValue(const Uni::Value& value) override
  {
    saveDeserial(value, "jointAngles", [](auto& v) { v.fill(0); }, jointAngles);
    saveDeserial(value, "headMatrixBuffer", [](auto& v) { v.reset(); }, headMatrixBuffer);
    saveDeserial(value, "sonarDist", [](auto& v) { v.fill(-1.f); }, sonarDist);
    saveDeserial(value, "sonarValid", [](auto& v) { v.fill(false); }, sonarValid);
    saveDeserial(value, "fsrLeft", [](auto& v) { v.fill(0); }, fsrLeft);
    saveDeserial(value, "fsrRight", [](auto& v) { v.fill(0); }, fsrRight);
    saveDeserial(value, "imu", [](auto& v) { v.fill(0); }, imu);
    saveDeserial(value, "switches", [](auto& v) { v.fill(0); }, switches);
    if (saveDeserial(value, "topImage", [](auto&) {}, image))
    {
      camera = Camera::TOP;
    }
    else if (saveDeserial(value, "bottomImage", [](auto& v) { v = ""; }, image))
    {
      camera = Camera::BOTTOM;
    }
    // TODO: Find a better default.
    saveDeserial(value, "timestamp", [](auto& v) { v = TimePoint::getCurrentTime(); }, timestamp);
  }

private:
  template <typename T, typename F>
  inline bool saveDeserial(const Uni::Value& value, const std::string& field, const F fallback,
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
  DataTypeName name = "ReplayConfigurations";
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
  DataTypeName name = "ReplayData";
  void reset() override {}

private:
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
