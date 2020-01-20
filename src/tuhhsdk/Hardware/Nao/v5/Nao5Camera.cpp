#include <cstdlib>
#include <cstring>
#include <linux/videodev2.h>
#include <poll.h>
#include <stdexcept>
#include <sys/fcntl.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <thread>
#include <unistd.h>

#include "Modules/Configuration/Configuration.h"
#include "print.h"

#include "Hardware/Nao/common/NaoCameraCommon.hpp"
#include "Nao5Camera.hpp"

Nao5Camera::Nao5Camera(const Camera camera)
  : NaoCamera(camera)
  , exposure_("exposure", V4L2_CID_EXPOSURE)
  , gamma_("gamma", V4L2_CID_GAMMA)
  , fadeToBlack_("fadeToBlack", V4L2_MT9M114_FADE_TO_BLACK)
  , aeMaxAGain_("aeMaxAGain", V4L2_MT9M114_AE_MAX_VIRT_AGAIN)
  , aeMinAGain_("aeMinAGain", V4L2_MT9M114_AE_MIN_VIRT_AGAIN)
  , aeMaxDGain_("aeMaxDGain", V4L2_MT9M114_AE_MAX_VIRT_DGAIN)
  , aeMinDGain_("aeMinDGain", V4L2_MT9M114_AE_MIN_VIRT_DGAIN)
  , aeTargetGain_("aeTargetGain", V4L2_MT9M114_AE_TARGET_GAIN)
  , brightnessDark_("brightnessDark", V4L2_MT9M114_BRIGHTNESS_DARK)
  , exposureAlgorithm_("exposureAlgorithm", V4L2_CID_EXPOSURE_ALGORITHM)
  , hFlip_("hFlip", V4L2_CID_HFLIP)
  , vFlip_("vFlip", V4L2_CID_VFLIP)
{
  cameraControlSettings_.push_back(&exposure_);
  cameraControlSettings_.push_back(&gamma_);
  cameraControlSettings_.push_back(&fadeToBlack_);
  cameraControlSettings_.push_back(&aeMaxAGain_);
  cameraControlSettings_.push_back(&aeMinAGain_);
  cameraControlSettings_.push_back(&aeMaxDGain_);
  cameraControlSettings_.push_back(&aeMinDGain_);
  cameraControlSettings_.push_back(&aeTargetGain_);
  cameraControlSettings_.push_back(&brightnessDark_);
  cameraControlSettings_.push_back(&exposureAlgorithm_);
  cameraControlSettings_.push_back(&hFlip_);
  cameraControlSettings_.push_back(&vFlip_);
}

Nao5Camera::~Nao5Camera() {}

void Nao5Camera::configure(Configuration& config, NaoInfo& naoInfo)
{
  config_ = &config;
  std::string device;
  std::string mountSuffix;
  device = (camera_ == Camera::TOP) ? "/dev/video0" : "/dev/video1";
  mountSuffix = "_v_5";

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

  if ((resolution_.x() % 16) != 0)
  {
    throw std::runtime_error(
        "The image width has to be divisible by 16 because of SSE-optimized readImage!");
  }

  setFormat();
  setFrameRate();

  createBuffers();

  for (auto& setting : cameraControlSettings_)
  {
    setting->initialize(fd_);
    setting->applyValue(config.get(mount_, setting->name).asInt32());
  }

  setSpecialControlSettings();

  verifyControlSettings();

  config.registerCallback(mount_, autoExposure_.name, [this](const Uni::Value& v) {
    autoExposure_.setConfiguredValue(v.asInt32());
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

  config.registerCallback(mount_, gamma_.name,
                          [this](const Uni::Value& v) { gamma_.applyValue(v.asInt32()); });

  config.registerCallback(mount_, fadeToBlack_.name,
                          [this](const Uni::Value& v) { fadeToBlack_.applyValue(v.asInt32()); });

  config.registerCallback(mount_, aeMaxAGain_.name, [this](const Uni::Value& v) {
    aeMaxAGain_.setConfiguredValue(v.asInt32());
    onExposureChange();
  });

  config.registerCallback(mount_, aeMinAGain_.name, [this](const Uni::Value& v) {
    aeMinAGain_.setConfiguredValue(v.asInt32());
    onExposureChange();
  });

  config.registerCallback(mount_, aeMaxDGain_.name, [this](const Uni::Value& v) {
    aeMaxDGain_.setConfiguredValue(v.asInt32());
    onExposureChange();
  });

  config.registerCallback(mount_, aeMinDGain_.name, [this](const Uni::Value& v) {
    aeMinDGain_.setConfiguredValue(v.asInt32());
    onExposureChange();
  });

  config.registerCallback(mount_, aeTargetGain_.name, [this](const Uni::Value& v) {
    aeTargetGain_.setConfiguredValue(v.asInt32());
    onExposureChange();
  });

  config.registerCallback(mount_, brightnessDark_.name, [this](const Uni::Value& v) {
    brightnessDark_.setConfiguredValue(v.asInt32());
    onExposureChange();
  });

  config.registerCallback(mount_, exposureAlgorithm_.name, [this](const Uni::Value& v) {
    exposureAlgorithm_.setConfiguredValue(v.asInt32());
    onExposureChange();
  });

  config.registerCallback(mount_, hFlip_.name,
                          [this](const Uni::Value& v) { hFlip_.applyValue(v.asInt32()); });

  config.registerCallback(mount_, vFlip_.name,
                          [this](const Uni::Value& v) { vFlip_.applyValue(v.asInt32()); });
}

void Nao5Camera::setControlSettings()
{
  for (auto& setting : cameraControlSettings_)
  {
    setting->applyValue();
  }
}

void Nao5Camera::setSpecialControlSettings() {}

void Nao5Camera::verifyControlSettings()
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

void Nao5Camera::onOrientationChange()
{
  // Nothing to do here for v5
}

void Nao5Camera::onExposureChange()
{
  autoExposure_.applyValue();
  exposure_.applyValue();
  brightness_.applyValue();
  brightnessDark_.applyValue();
  exposureAlgorithm_.applyValue();
  aeTargetGain_.applyValue();
  aeMinAGain_.applyValue();
  aeMaxAGain_.applyValue();
  aeMinDGain_.applyValue();
  aeMaxDGain_.applyValue();
}

void Nao5Camera::onHueChange()
{
  hue_.applyValue();
}
