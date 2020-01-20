#include <linux/usb/video.h>
#include <linux/uvcvideo.h>

#include <cstdlib>
#include <cstring>
#include <linux/videodev2.h>
#include <poll.h>
#include <stdexcept>
#include <fcntl.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <thread>
#include <unistd.h>

#include "Modules/Configuration/Configuration.h"
#include "Tools/Storage/UniValue/UniValue.h"
#include "print.h"

#include "Hardware/Nao/common/NaoCameraCommon.hpp"
#include "Nao6Camera.hpp"

Nao6Camera::Nao6Camera(const Camera camera)
  : NaoCamera(camera)
  , exposure_("exposure", V4L2_CID_EXPOSURE_ABSOLUTE)
  , autoHue_("autoHue", V4L2_CID_HUE_AUTO)
  , autoFocus_("autoFocus", V4L2_CID_FOCUS_AUTO)
  , focus_("focus", V4L2_CID_FOCUS_ABSOLUTE)
{
  cameraControlSettings_.push_back(&exposure_);
  cameraControlSettings_.push_back(&autoHue_);
  cameraControlSettings_.push_back(&autoFocus_);
  cameraControlSettings_.push_back(&focus_);
}

Nao6Camera::~Nao6Camera() {}

void Nao6Camera::configure(Configuration& config, NaoInfo& naoInfo)
{
  config_ = &config;
  std::string device;
  std::string mountSuffix;
  device = (camera_ == Camera::TOP) ? "/dev/video-top" : "/dev/video-bottom";
  mountSuffix = "_v_6";

  fd_ = open(device.c_str(), O_RDWR | O_NONBLOCK);
  if (fd_ < 0)
  {
    throw std::runtime_error("Could not open camera device file!");
  }

  naoInfo_ = naoInfo;

  config.mount(mount_, mount_ + mountSuffix + ".json", ConfigurationType::HEAD);

  config.get(mount_, "bufferCount") >> bufferCount_;
  config.get(mount_, "fps") >> fps_;
  config.get(mount_, "resolution") >> resolution_;

  config.get(mount_, "enableDigitalEffects") >> enableDigitalEffects_;
  config.get(mount_, "enableAWBBias") >> enableAWBBias_;
  config.get(mount_, "registerAddr") >> registerAddr_;
  config.get(mount_, "registerValue") >> registerValue_;
  config.get(mount_, "registerWrite") >> registerWrite_;


  if ((resolution_.x() % 16) != 0)
  {
    throw std::runtime_error(
        "The image width has to be divisible by 16 because of SSE-optimized readImage!");
  }

  setFormat();
  setFrameRate();

  createBuffers();

  // as of now we need to have VIDIOC_STREAMON during registerRead() so we have to startCapture
  // during setControlSettings
  startCapture();
  std::this_thread::sleep_for(std::chrono::milliseconds(200));

  // The register 0x5001 must never be 0. It contains some bits that are for internal camera debug
  // modes only (and are set to 1 by default), so it is safe to assume that readRegister(0x5001) is
  // greater than 0
  assert(readRegister(0x5001) > 0 && "Camera register 0x5001 contains garbage. Either camera reset "
                                     "was not successful or register actions are faulty");

  // Initialize settings that are only available via UVC UX (setting cam registers in our case).
  onDigitalEffectsChange();
  // Wait one frame for the register stuff to settle
  std::this_thread::sleep_for(std::chrono::milliseconds(34));
  onAWBBiasChange();
  // Wait one frame for the register stuff to settle
  std::this_thread::sleep_for(std::chrono::milliseconds(34));

  // stop capture for now.
  stopCapture();
  clearBuffers();
  std::this_thread::sleep_for(std::chrono::milliseconds(100));

  createBuffers();

  for (auto& setting : cameraControlSettings_)
  {
    setting->initialize(fd_);
    setting->applyValue(config.get(mount_, setting->name).asInt32());
  }

  setSpecialControlSettings();

  verifyControlSettings();


  config.registerCallback(mount_, autoExposure_.name, [this](const Uni::Value& v) {
    autoExposure_.applyValue(v.asInt32());
    onExposureChange();
  });

  config.registerCallback(mount_, autoWhiteBalance_.name, [this](const Uni::Value& v) {
    autoWhiteBalance_.setConfiguredValue(v.asInt32());
    onWhiteBalanceTemperatureChange();
  });

  config.registerCallback(mount_, brightness_.name, [this](const Uni::Value& v) {
    brightness_.setConfiguredValue(v.asInt32());
    onExposureChange();
  });

  config.registerCallback(mount_, contrast_.name,
                          [this](const Uni::Value& v) { contrast_.applyValue(v.asInt32()); });

  config.registerCallback(mount_, exposure_.name, [this](const Uni::Value& v) {
    exposure_.setConfiguredValue(v.asInt32());
    onExposureChange();
  });

  config.registerCallback(mount_, gain_.name, [this](const Uni::Value& v) {
    gain_.setConfiguredValue(v.asInt32());
    onExposureChange();
  });

  config.registerCallback(mount_, hue_.name, [this](const Uni::Value& v) {
    hue_.setConfiguredValue(v.asInt32());
    onHueChange();
  });

  config.registerCallback(mount_, saturation_.name,
                          [this](const Uni::Value& v) { saturation_.applyValue(v.asInt32()); });

  config.registerCallback(mount_, sharpness_.name,
                          [this](const Uni::Value& v) { sharpness_.applyValue(v.asInt32()); });

  config.registerCallback(mount_, whiteBalanceTemperature_.name, [this](const Uni::Value& v) {
    whiteBalanceTemperature_.setConfiguredValue(v.asInt32());
    onWhiteBalanceTemperatureChange();
  });

  config.registerCallback(mount_, autoFocus_.name, [this](const Uni::Value& v) {
    autoFocus_.setConfiguredValue(v.asInt32());
    onFocusChange();
  });

  config.registerCallback(mount_, autoHue_.name, [this](const Uni::Value& v) {
    autoHue_.setConfiguredValue(v.asInt32());
    onHueChange();
  });

  config.registerCallback(mount_, "enableDigitalEffects", [this](const Uni::Value& v) {
    v >> enableDigitalEffects_;
    onDigitalEffectsChange();
  });

  config.registerCallback(mount_, "enableAWBBias", [this](const Uni::Value& v)
  {
    v >> enableAWBBias_;
    onAWBBiasChange();
  });

  config.registerCallback(mount_, focus_.name, [this](const Uni::Value& v) {
    focus_.applyValue(v.asInt32());
    onFocusChange();
  });

  config.registerCallback(mount_, "registerAddr",
                          [this](const Uni::Value& v) { v >> registerAddr_; });

  config.registerCallback(mount_, "registerValue",
                          [this](const Uni::Value& v) { v >> registerValue_; });

  config.registerCallback(mount_, "registerWrite", [this](const Uni::Value& v) {
    v >> registerWrite_;
    onRegisterAction();
  });
}

std::uint16_t Nao6Camera::readRegister(std::uint16_t addr) const
{
  // construct the query struct
  uvc_xu_control_query xu_query;
  std::memset(&xu_query, 0, sizeof(xu_query));
  xu_query.unit = 3;
  // selecting register control on the microcontroller?
  xu_query.selector = 0x0e;
  xu_query.query = UVC_SET_CUR;
  xu_query.size = 5;

  // contruct the data block
  std::uint8_t data[5];
  std::memset(data, 0, 5);
  // set flag to "Read"
  data[0] = 0;
  // split 16-bit address into two 8-bit parts
  data[1] = addr >> 8;
  data[2] = addr & 0xff;
  xu_query.data = data;
  if (-1 == ioctl(fd_, UVCIOC_CTRL_QUERY, &xu_query))
  {
    Log(LogLevel::ERROR) << "(ERROR) : UVC_SET_CUR fails: " << std::strerror(errno);
    assert(false);
  }

  // wait for the microcontroller to query the register from the camera
  std::this_thread::sleep_for(std::chrono::milliseconds(500));

  // query the value
  xu_query.query = UVC_GET_CUR;
  if (-1 == ioctl(fd_, UVCIOC_CTRL_QUERY, &xu_query))
  {
    Log(LogLevel::ERROR) << "(ERROR) : UVC_GET_CUR fails: " << std::strerror(errno);
    assert(false);
  }

  // return the concatenated value
  return (std::uint16_t(data[3]) << 8) | std::uint16_t(data[4]);
}

void Nao6Camera::writeRegister(std::uint16_t addr, std::uint16_t value) const
{
  // construct the query struct
  struct uvc_xu_control_query xu_query;
  std::memset(&xu_query, 0, sizeof(xu_query));
  xu_query.unit = 3;
  // selecting register control on the microcontroller?
  xu_query.selector = 0x0e;
  xu_query.query = UVC_SET_CUR;
  xu_query.size = 5;

  std::uint8_t data[5];
  std::memset(data, 0, 5);
  // set flag to "Write"
  data[0] = 1;
  // split 16-bit address into two 8-bit parts
  data[1] = addr >> 8;
  data[2] = addr & 0xff;
  // split 16-bit value into two 8-bit parts
  data[3] = value >> 8;
  data[4] = value & 0xff;
  xu_query.data = data;
  if (-1 == ioctl(fd_, UVCIOC_CTRL_QUERY, &xu_query))
  {
    Log(LogLevel::ERROR) << "(ERROR) : UVC_SET_CUR fails: " << std::strerror(errno);
    assert(false);
  }
}

void Nao6Camera::onFocusChange()
{
  autoFocus_.applyValue();
  focus_.applyValue();
}

void Nao6Camera::onOrientationChange()
{
  switch (camera_)
  {
    case Camera::TOP:
    {
      __u16 value = 1;
      uvc_xu_control_query xu;
      xu.unit = 3;
      xu.selector = 0x0c; // Horizontal flip
      xu.query = UVC_SET_CUR;
      xu.size = sizeof(value);
      xu.data = reinterpret_cast<__u8*>(&value);
      ioctl(fd_, UVCIOC_CTRL_QUERY, &xu);
      xu.selector = 0x0d; // Vertical flip
      ioctl(fd_, UVCIOC_CTRL_QUERY, &xu);
      break;
    }
    case Camera::BOTTOM:
    {
      __u16 value = 0;
      uvc_xu_control_query xu;
      xu.unit = 3;
      xu.selector = 0x0c;
      xu.query = UVC_SET_CUR;
      xu.size = sizeof(value);
      xu.data = reinterpret_cast<__u8*>(&value);
      ioctl(fd_, UVCIOC_CTRL_QUERY, &xu);
      xu.selector = 0x0d;
      ioctl(fd_, UVCIOC_CTRL_QUERY, &xu);
      break;
    }
    default:
      break;
  }
}

void Nao6Camera::onExposureChange()
{
  brightness_.applyValue();
  autoExposure_.applyValue();
  exposure_.applyValue();
  gain_.applyValue();
}

void Nao6Camera::onHueChange()
{
  autoHue_.applyValue();
  hue_.applyValue();
}

void Nao6Camera::onRegisterAction()
{
  if (registerWrite_)
  {
    Log(LogLevel::INFO) << "WRITE_REGISTER " << std::to_string(registerAddr_)
                        << " VALUE = " << std::to_string(registerValue_);
    writeRegister(static_cast<std::uint16_t>(registerAddr_),
                  static_cast<std::uint16_t>(registerValue_));
  }
  else
  {
    Log(LogLevel::INFO) << "READ_REGISTER ADDR: " << std::to_string(registerAddr_);
    const std::uint16_t newValue = readRegister(static_cast<std::uint16_t>(registerAddr_));
    config_->set(mount_, "registerValue", Uni::Value(static_cast<int32_t>(newValue)));
  }
}

void Nao6Camera::onDigitalEffectsChange()
{
  const std::uint16_t reg = 0x5001;
  const std::uint16_t bit = 7;
  std::uint16_t registerValue = readRegister(reg);

  setSingleBit(registerValue, bit, enableDigitalEffects_);

  writeRegister(reg, registerValue);
}

void Nao6Camera::onAWBBiasChange()
{
  const std::uint16_t reg = 0x5005;
  const std::uint16_t bit = 5;
  std::uint16_t registerValue = readRegister(reg);

  setSingleBit(registerValue, bit, enableAWBBias_);

  writeRegister(reg, registerValue);
}

void Nao6Camera::setControlSettings()
{
  for (auto& setting : cameraControlSettings_)
  {
    setting->applyValue();
  }
}


void Nao6Camera::setSpecialControlSettings()
{
  onOrientationChange();
}

void Nao6Camera::verifyControlSettings()
{
  for (auto& setting : cameraControlSettings_)
  {
    if (!setting->isAppliedGracefully())
    {
      Log(LogLevel::ERROR) << "Setting \"" << setting->name << "\" altered from configured value";
      assert(false);
    }
  }
}

void Nao6Camera::setSingleBit(std::uint16_t& value, std::uint8_t bit, bool enable)
{
  // This sets the nth bit of value to "enable"
  // See: https://stackoverflow.com/questions/47981/how-do-you-set-clear-and-toggle-a-single-bit
  value =  value ^ ((-static_cast<uint16_t>(enable ? 1 : 0) ^ value) & (1 << bit));
}
