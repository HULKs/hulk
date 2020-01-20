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
#include "Tools/Math/Range.hpp"
#include "print.h"

#include "NaoCamera.hpp"
#include "NaoCameraCommon.hpp"

NaoCamera::NaoCamera(const Camera camera)
  : camera_(camera)
  , config_(nullptr)
  , mount_((camera_ == Camera::TOP) ? "topCamera" : "bottomCamera")
  , fd_(-1)
  , fps_(0)
  , bufferCount_(0)
  , buffersInitialized_(false)
  , resolution_(0, 0)
  , autoExposure_("autoExposure", V4L2_CID_EXPOSURE_AUTO)
  , autoWhiteBalance_("autoWhiteBalance", V4L2_CID_AUTO_WHITE_BALANCE)
  , brightness_("brightness", V4L2_CID_BRIGHTNESS)
  , contrast_("contrast", V4L2_CID_CONTRAST)
  , gain_("gain", V4L2_CID_GAIN)
  , hue_("hue", V4L2_CID_HUE)
  , saturation_("saturation", V4L2_CID_SATURATION)
  , sharpness_("sharpness", V4L2_CID_SHARPNESS)
  , whiteBalanceTemperature_("whiteBalanceTemperature", V4L2_CID_WHITE_BALANCE_TEMPERATURE, 0, true)
{
  std::memset(&currentBuffer_, 0, sizeof(currentBuffer_));
  currentBuffer_.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
  currentBuffer_.memory = V4L2_MEMORY_MMAP;

  cameraControlSettings_.push_back(&autoExposure_);
  cameraControlSettings_.push_back(&autoWhiteBalance_);
  cameraControlSettings_.push_back(&brightness_);
  cameraControlSettings_.push_back(&contrast_);
  cameraControlSettings_.push_back(&gain_);
  cameraControlSettings_.push_back(&hue_);
  cameraControlSettings_.push_back(&saturation_);
  cameraControlSettings_.push_back(&sharpness_);
  cameraControlSettings_.push_back(&whiteBalanceTemperature_);

  imageValid = false;
  timestamp = 0;
}

NaoCamera::~NaoCamera()
{
  clearBuffers();
  close(fd_);
}

float NaoCamera::waitForImage()
{
  // The waiting now happens for both cameras at the same time
  return 0;
}

bool NaoCamera::waitForCameras(std::array<NaoCamera*, 2> cameras, int timeout)
{
  pollfd pollfds[cameras.size()];
  for (std::size_t i = 0; i < cameras.size(); ++i)
  {
    // Only poll cameras without valid image
    int fd = cameras[i]->imageValid ? -1 : cameras[i]->fd_;
    pollfds[i] = {fd, POLLIN | POLLPRI, 0};
  }

  int polled = poll(pollfds, cameras.size(), timeout);
  if (polled < 0)
  {
    Log(LogLevel::ERROR) << "Unable to fetch images. Image poll returned -1 (Error)";
    assert(false);
    return false;
  }
  else if (polled == 0)
  {
    Log(LogLevel::ERROR) << "Unable to fetch images. Image poll returned 0 (poll timed out)";
    return false;
  }

  for (std::size_t i = 0; i < cameras.size(); ++i)
  {
    if (pollfds[i].revents & POLLIN)
    {
      v4l2_buffer lastBuffer;
      bool isFirstImage = true;

      while (ioctl(cameras[i]->fd_, VIDIOC_DQBUF, &cameras[i]->currentBuffer_) == 0)
      {
        if (isFirstImage)
        {
          isFirstImage = false;
        }
        else
        {
          // Drop image if there is a newer one
          if (ioctl(cameras[i]->fd_, VIDIOC_QBUF, &lastBuffer) < 0)
          {
            throw std::runtime_error("Unable to requeue the buffer");
          }
          Log(LogLevel::WARNING) << "Dropped a frame";
        }
        lastBuffer = cameras[i]->currentBuffer_;
      }

      // errno is EAGAIN if the nonblocking VIDIOC_DQBUF returned without an image availabe.
      // So after removing all waiting images from the queue the queue should be empty
      // and thus the errno should be EAGAIN
      if (errno != EAGAIN)
      {
        Log(LogLevel::ERROR) << "VIDEOC_DQBUF is != EAGAIN. No image available";
        return false;
      }
      else
      {
        // V4L2 gives the time at which the first pixel of the image was recorded as timeval
        // "+ i * 1000": This is a hack! When top and bottom camera do have the same timestamp
        //               one of them will be skipped in our current debug protocol impl.
        cameras[i]->timestamp =
            static_cast<__u64>(cameras[i]->currentBuffer_.timestamp.tv_sec) * 1000000ll +
            cameras[i]->currentBuffer_.timestamp.tv_usec + i * 1000;
        // This fix is needed as the first image that we get on the v6 hardware has a timestamp
        // that does not make any sense (to @rkost, @nagua).
        cameras[i]->imageValid = cameras[i]->timestamp >= TimePoint::getBaseTime();
        if (!cameras[i]->imageValid)
        {
          Log(LogLevel::WARNING) << "Camera timestamp smaller than base time (normal during the "
                                    "first second(s)). Skipping image";
          // We need to queue the current buffer again as we marked the image as invalid. This would
          // cause releaseImage() to not queue the buffer again, thus the camera is not able to
          // capture images anymore.
          if (ioctl(cameras[i]->fd_, VIDIOC_QBUF, &cameras[i]->currentBuffer_) < 0)
          {
            throw std::runtime_error("Unable to queue buffer.");
          }
        }
      }
    }
    else if (pollfds[i].revents)
    {
      Log(LogLevel::ERROR) << "Camera is in an unknown state (This is really bad).";
      assert(false && "Strange camera error perhaps add automatic camera resetting");
      return false;
    }
  }
  return true;
}

TimePoint NaoCamera::readImage(Image422& image)
{
  image.setData(reinterpret_cast<YCbCr422*>(bufferMem_[currentBuffer_.index]), resolution_);

  const unsigned int millisecondsSince1970 = timestamp / 1000;
  return TimePoint(millisecondsSince1970 - TimePoint::getBaseTime());
}

void NaoCamera::releaseImage()
{
  if (imageValid)
  {
    if (ioctl(fd_, VIDIOC_QBUF, &currentBuffer_) < 0)
    {
      throw std::runtime_error("Unable to queue buffer");
    }
    imageValid = false;
  }
}

void NaoCamera::startCapture()
{
  Log(LogLevel::INFO) << "Starting capture for camera " << static_cast<int>(camera_);
  v4l2_buf_type type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
  if (ioctl(fd_, VIDIOC_STREAMON, &type) < 0)
  {
    throw std::runtime_error("Could not start image capturing in NaoCamera!");
  }
}

void NaoCamera::stopCapture()
{
  Log(LogLevel::INFO) << "Stopping capture for camera " << static_cast<int>(camera_);
  v4l2_buf_type type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
  if (ioctl(fd_, VIDIOC_STREAMOFF, &type) < 0)
  {
    throw std::runtime_error("Could not stop image capturing in NaoCamera!");
  }
}

Camera NaoCamera::getCameraType()
{
  return camera_;
}

void NaoCamera::setFormat()
{
  v4l2_format fmt;
  memset(&fmt, 0, sizeof(fmt));
  fmt.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
  fmt.fmt.pix.width = resolution_.x();
  fmt.fmt.pix.height = resolution_.y();
  fmt.fmt.pix.pixelformat = V4L2_PIX_FMT_YUYV;
  fmt.fmt.pix.field = V4L2_FIELD_NONE;
  fmt.fmt.pix.bytesperline = 2 * fmt.fmt.pix.width * fmt.fmt.pix.height;
  if (ioctl(fd_, VIDIOC_S_FMT, &fmt) < 0)
  {
    throw std::runtime_error("Could not set image format in NaoCamera!");
  }
  if ((fmt.type != V4L2_BUF_TYPE_VIDEO_CAPTURE) || (fmt.fmt.pix.width != (__u32)resolution_.x()) ||
      (fmt.fmt.pix.height != (__u32)resolution_.y()) ||
      (fmt.fmt.pix.pixelformat != V4L2_PIX_FMT_YUYV) || (fmt.fmt.pix.field != V4L2_FIELD_NONE))
  {
    throw std::runtime_error(
        "Could set image format but the driver does not accept the settings in NaoCamera!");
  }
}

void NaoCamera::setFrameRate()
{
  v4l2_streamparm fps;
  memset(&fps, 0, sizeof(fps));
  fps.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;

  if (ioctl(fd_, VIDIOC_G_PARM, &fps))
  {
    throw std::runtime_error("Could not read frame rate in NaoCamera!");
  }

  fps.parm.capture.timeperframe.numerator = 1;
  fps.parm.capture.timeperframe.denominator = fps_;

  if (ioctl(fd_, VIDIOC_S_PARM, &fps) < 0)
  {
    throw std::runtime_error("Could not set frame rate in NaoCamera!");
  }
  if ((fps.type != V4L2_BUF_TYPE_VIDEO_CAPTURE) || (fps.parm.capture.timeperframe.numerator != 1) ||
      (fps.parm.capture.timeperframe.denominator != fps_))
  {
    throw std::runtime_error(
        "Could set frame rate but the driver does not accept the settings in NaoCamera!");
  }
}

void NaoCamera::createBuffers()
{
  v4l2_buffer buf;
  v4l2_requestbuffers reqbufs;

  bufferMem_.resize(bufferCount_);
  bufferLength_.resize(bufferCount_);

  memset(&reqbufs, 0, sizeof(reqbufs));
  reqbufs.count = bufferCount_;
  reqbufs.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
  reqbufs.memory = V4L2_MEMORY_MMAP;
  if (ioctl(fd_, VIDIOC_REQBUFS, &reqbufs) < 0)
  {
    throw std::runtime_error("Could not request buffers from driver in NaoCamera!");
  }
  for (unsigned int i = 0; i < bufferCount_; i++)
  {
    memset(&buf, 0, sizeof(buf));
    buf.index = i;
    buf.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
    if (ioctl(fd_, VIDIOC_QUERYBUF, &buf) < 0)
    {
      for (--i; i >= 0; i--)
      {
        munmap(bufferMem_[i], bufferLength_[i]);
      }
      throw std::runtime_error("Could not get buffer in NaoCamera!");
    }
    bufferLength_[i] = buf.length;
    bufferMem_[i] = (unsigned char*)mmap(0, bufferLength_[i], PROT_READ | PROT_WRITE, MAP_SHARED,
                                         fd_, buf.m.offset);
    if (bufferMem_[i] == MAP_FAILED)
    {
      for (--i; i >= 0; i--)
      {
        munmap(bufferMem_[i], bufferLength_[i]);
      }
      throw std::runtime_error("Could not map buffer in NaoCamera!");
    }
    if (ioctl(fd_, VIDIOC_QBUF, &buf) < 0)
    {
      for (; i >= 0; i--)
      {
        munmap(bufferMem_[i], bufferLength_[i]);
      }
      throw std::runtime_error("Could not enqueue buffer in NaoCamera!");
    }
  }
  buffersInitialized_ = true;
}

void NaoCamera::clearBuffers()
{
  if (!buffersInitialized_)
  {
    return;
  }
  // unmap buffers
  for (unsigned int i = 0; i < bufferCount_; i++)
  {
    if (bufferMem_[i])
    {
      munmap(bufferMem_[i], bufferLength_[i]);
      bufferMem_[i] = nullptr;
    }
  }
  bufferMem_.clear();
  bufferLength_.clear();

  buffersInitialized_ = false;
}

void NaoCamera::onWhiteBalanceTemperatureChange()
{
  autoWhiteBalance_.applyValue();
  // This will fail silently on v5 if auto white balance is enabled...
  whiteBalanceTemperature_.applyValue();
}

NaoCamera::V4L2CtrlSetting::V4L2CtrlSetting(const std::string name, const int command,
                                            int configuredValue, bool acceptFailure)
  : name(name)
  , command(command)
  , configuredValue_(configuredValue)
  , acceptFailure_(acceptFailure)
{
}

void NaoCamera::V4L2CtrlSetting::initialize(int fd, unsigned int retries)
{
  assert(fd >= 0);
  fd_ = fd;

  v4l2_queryctrl qctrl;
  for (unsigned int retry = 0; retry < retries; retry++)
  {
    memset(&qctrl, 0, sizeof(qctrl));
    qctrl.id = command;

    // Query the current state for this control setting
    int ret = ioctl(fd_, VIDIOC_QUERYCTRL, &qctrl);
    if (ret < 0)
    {
      Log(LogLevel::WARNING) << "Failed to query camera setting for control setting \"" << name
                             << "\". ioctl returned " << ret << ". Retrying...";
      // Wait for one frame.
      std::this_thread::sleep_for(std::chrono::milliseconds(34));
      continue;
    }
    else
    {
      // Check if control setting is PERMANENTLY disabled by the camera device
      if (qctrl.flags & V4L2_CTRL_FLAG_DISABLED)
      {
        Log(LogLevel::ERROR) << "Camera control setting \"" << name
                             << "\" is permanently disabled.";
        assert(false);
      }
      // Check if control setting is of an supported type
      if (qctrl.type != V4L2_CTRL_TYPE_BOOLEAN && qctrl.type != V4L2_CTRL_TYPE_INTEGER &&
          qctrl.type != V4L2_CTRL_TYPE_MENU)
      {
        Log(LogLevel::ERROR) << "Camera setting \"" << name << "\" is unsupported";
        assert(false);
      }

      setCameraBounds(qctrl.minimum, qctrl.maximum, qctrl.step);
      return;
    }
  }
  Log(LogLevel::ERROR) << "Unable to query camera setting for control setting \"" << name << "\".";
  throw std::runtime_error("Unable to initialize camera.");
}


bool NaoCamera::V4L2CtrlSetting::isValid(int value) const
{
  return value >= min_ && value <= max_;
}

int NaoCamera::V4L2CtrlSetting::clipToRangeAndStep(int value) const
{
  // ensure that we only set multiple values of "step" counting from "min_"
  const int stepped = (step_ * ((value - min_) / step_)) + min_;
  assert(stepped % step_ == 0);
  if (value != stepped)
  {
    Log(LogLevel::WARNING) << "Value " << value << " for " << name << " is illegal (step = " << step_
                           << "). Falling back to " << stepped;
  }
  // ensure that the value is inside the bounds
  const int clipped = Range<int>::clipToGivenRange(stepped, min_, max_);
  assert(clipped <= max_ && clipped >= min_ && clipped % step_ == 0);
  if (stepped != clipped)
  {
    Log(LogLevel::WARNING) << "Value " << stepped << " for " << name << " is illegal (bounds = ["
                           << min_ << ", " << max_ << "]). Falling back to " << clipped;
  }
  return clipped;
}

bool NaoCamera::V4L2CtrlSetting::isApplied()
{
  return configuredValue_ == getAppliedValue();
}

bool NaoCamera::V4L2CtrlSetting::isAppliedGracefully()
{
  return acceptFailure_ ? true : isApplied();
}

bool NaoCamera::V4L2CtrlSetting::applyValue(int value, unsigned int retries)
{
  configuredValue_ = clipToRangeAndStep(value);
  Log(LogLevel::INFO) << "Setting camera control setting \"" << name << "\" to value "
                      << configuredValue_;

  v4l2_control ctrl;
  bool applied = false;
  for (unsigned int retry = 0; retry < retries; retry++)
  {
    memset(&ctrl, 0, sizeof(ctrl));

    ctrl.id = command;
    ctrl.value = configuredValue_;
    if (ioctl(fd_, VIDIOC_S_CTRL, &ctrl) < 0)
    {
      Log(LogLevel::WARNING) << "Failed to set setting \"" << name << "\" to value "
                             << configuredValue_ << " on try no " << retry << ". Retrying...";
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
    Log(LogLevel::ERROR) << "Failed to set setting \"" << name << "\" to value "
                         << configuredValue_;
  }
  if (acceptFailure_ && !applied)
  {
    Log(LogLevel::WARNING) << "Ignoring the fact that \"" << name << "\" could not be set...";
    return true;
  }
  assert(applied);
  return applied;
}

bool NaoCamera::V4L2CtrlSetting::applyValue(unsigned int retries)
{
  return applyValue(configuredValue_, retries);
}

void NaoCamera::V4L2CtrlSetting::setConfiguredValue(int value)
{
  configuredValue_ = value;
}

int NaoCamera::V4L2CtrlSetting::getAppliedValue(unsigned int retries)
{
  v4l2_control ctrl;
  for (unsigned int retry = 0; retry < retries; retry++)
  {
    memset(&ctrl, 0, sizeof(ctrl));
    ctrl.id = command;

    if (ioctl(fd_, VIDIOC_G_CTRL, &ctrl) < 0)
    {
      Log(LogLevel::WARNING) << "Unable to read setting \"" << name << "\""
                             << " on try no " << retry << ". Retrying...";
      // wait one frame (30FPS)
      std::this_thread::sleep_for(std::chrono::milliseconds(34));
      continue;
    }
    appliedValue_ = ctrl.value;
    Log(LogLevel::DEBUG) << "Control setting \"" << name << "\" is set to " << appliedValue_;
    return appliedValue_;
  }
  Log(LogLevel::ERROR) << "Unable to read setting \"" << name << "\"";
  assert(false);
  return 0;
}

int NaoCamera::V4L2CtrlSetting::getConfiguredValue() const
{
  return configuredValue_;
}

void NaoCamera::V4L2CtrlSetting::setCameraBounds(const int min, const int max, const int step)
{
  min_ = min;
  max_ = max;
  step_ = step;
  assert(step > 0);
  Log(LogLevel::DEBUG) << "Bounds for control setting \"" << name << "\" are [" << min_ << ", "
                       << max_ << "]. Step is " << step_;
}
