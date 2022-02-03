#include "Hardware/Nao/V4L2CtrlSetting.hpp"
#include "Framework/Log/Log.hpp"
#include <cassert>
#include <cstring>
#include <linux/videodev2.h>
#include <sys/ioctl.h>
#include <thread>
#include <utility>

V4L2CtrlSetting::V4L2CtrlSetting(const int fd, std::string name, const int command,
                                 int configuredValue, const bool acceptFailure,
                                 const unsigned int retries)
  : name_(std::move(name))
  , command_(command)
  , fd_(fd)
  , configuredValue_(configuredValue)
  , acceptFailure_(acceptFailure)
{
  assert(fd >= 0);
  v4l2_queryctrl qctrl{};
  for (unsigned int retry = 0; retry < retries; retry++)
  {
    std::memset(&qctrl, 0, sizeof(qctrl));
    qctrl.id = command;

    // Query the current state for this control setting
    int ret = ioctl(fd_, VIDIOC_QUERYCTRL, &qctrl);
    if (ret < 0)
    {
      Log<M_TUHHSDK>(LogLevel::WARNING) << "Failed to query camera setting for control setting \""
                                        << name_ << "\". ioctl returned " << ret << ". Retrying...";
      // Wait for one frame.
      std::this_thread::sleep_for(std::chrono::milliseconds(34));
      continue;
    }
    // Check if control setting is PERMANENTLY disabled by the camera device
    if ((qctrl.flags & V4L2_CTRL_FLAG_DISABLED) != 0)
    {
      Log<M_TUHHSDK>(LogLevel::ERROR)
          << "Camera control setting \"" << name_ << "\" is permanently disabled.";
      assert(false);
    }
    // Check if control setting is of an supported type
    if (qctrl.type != V4L2_CTRL_TYPE_BOOLEAN && qctrl.type != V4L2_CTRL_TYPE_INTEGER &&
        qctrl.type != V4L2_CTRL_TYPE_MENU)
    {
      Log<M_TUHHSDK>(LogLevel::ERROR) << "Camera setting \"" << name_ << "\" is unsupported";
      assert(false);
    }

    setCameraBounds(qctrl.minimum, qctrl.maximum, qctrl.step);
    return;
  }
  Log<M_TUHHSDK>(LogLevel::ERROR) << "Unable to query camera setting for control setting \""
                                  << name_ << "\".";
  throw std::runtime_error("Unable to initialize camera.");
}

bool V4L2CtrlSetting::isValid(int value) const
{
  return value >= min_ && value <= max_;
}

int V4L2CtrlSetting::clipToRangeAndStep(int value) const
{
  // ensure that we only set multiple values of "step" counting from "min_"
  const int stepped = (step_ * ((value - min_) / step_)) + min_;
  assert(stepped % step_ == 0);
  if (value != stepped)
  {
    Log<M_TUHHSDK>(LogLevel::WARNING)
        << "Value " << value << " for " << name_ << " is illegal (step = " << step_
        << "). Falling back to " << stepped;
  }
  // ensure that the value is inside the bounds
  const int clipped = std::clamp(stepped, min_, max_);
  assert(clipped <= max_ && clipped >= min_ && clipped % step_ == 0);
  if (stepped != clipped)
  {
    Log<M_TUHHSDK>(LogLevel::WARNING)
        << "Value " << stepped << " for " << name_ << " is illegal (bounds = [" << min_ << ", "
        << max_ << "]). Falling back to " << clipped;
  }
  return clipped;
}

bool V4L2CtrlSetting::isApplied()
{
  return configuredValue_ == getAppliedValue();
}

bool V4L2CtrlSetting::isAppliedGracefully()
{
  return acceptFailure_ ? true : isApplied();
}

bool V4L2CtrlSetting::applyValue(int value, unsigned int retries)
{
  configuredValue_ = clipToRangeAndStep(value);
  Log<M_TUHHSDK>(LogLevel::INFO) << "Setting camera control setting \"" << name_ << "\" to value "
                                 << configuredValue_;

  v4l2_control ctrl{};
  bool applied = false;
  for (unsigned int retry = 0; retry < retries; retry++)
  {
    memset(&ctrl, 0, sizeof(ctrl));

    ctrl.id = command_;
    ctrl.value = configuredValue_;
    if (ioctl(fd_, VIDIOC_S_CTRL, &ctrl) < 0)
    {
      Log<M_TUHHSDK>(LogLevel::WARNING)
          << "Failed to set setting \"" << name_ << "\" to value " << configuredValue_
          << " on try no " << retry << ". Retrying...";
      std::this_thread::sleep_for(std::chrono::milliseconds(17));
      continue;
    }
    applied = isApplied();
    if (applied)
    {
      break;
    }
  }

  if (!applied)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR)
        << "Failed to set setting \"" << name_ << "\" to value " << configuredValue_;
  }
  if (acceptFailure_ && !applied)
  {
    Log<M_TUHHSDK>(LogLevel::WARNING)
        << "Ignoring the fact that \"" << name_ << "\" could not be set...";
    return true;
  }
  assert(applied);
  return applied;
}

bool V4L2CtrlSetting::applyValue(unsigned int retries)
{
  return applyValue(configuredValue_, retries);
}

void V4L2CtrlSetting::setConfiguredValue(int value)
{
  configuredValue_ = value;
}

int V4L2CtrlSetting::getAppliedValue(unsigned int retries)
{
  v4l2_control ctrl{};
  for (unsigned int retry = 0; retry < retries; retry++)
  {
    memset(&ctrl, 0, sizeof(ctrl));
    ctrl.id = command_;

    if (ioctl(fd_, VIDIOC_G_CTRL, &ctrl) < 0)
    {
      Log<M_TUHHSDK>(LogLevel::WARNING) << "Unable to read setting \"" << name_ << "\""
                                        << " on try no " << retry << ". Retrying...";
      // wait one frame (30FPS)
      std::this_thread::sleep_for(std::chrono::milliseconds(34));
      continue;
    }
    appliedValue_ = ctrl.value;
    Log<M_TUHHSDK>(LogLevel::DEBUG)
        << "Control setting \"" << name_ << "\" is set to " << appliedValue_;
    return appliedValue_;
  }
  Log<M_TUHHSDK>(LogLevel::ERROR) << "Unable to read setting \"" << name_ << "\"";
  assert(false);
  return 0;
}

int V4L2CtrlSetting::getConfiguredValue() const
{
  return configuredValue_;
}

const std::string& V4L2CtrlSetting::getName() const
{
  return name_;
}

void V4L2CtrlSetting::setCameraBounds(const int min, const int max, const int step)
{
  min_ = min;
  max_ = max;
  step_ = step;
  assert(step > 0);
  Log<M_TUHHSDK>(LogLevel::DEBUG) << "Bounds for control setting \"" << name_ << "\" are [" << min_
                                  << ", " << max_ << "]. Step is " << step_;
}
