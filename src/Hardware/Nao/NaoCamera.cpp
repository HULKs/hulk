#include "Hardware/Nao/NaoCamera.hpp"
#include "Data/CycleInfo.hpp"
#include "Framework/Configuration/Configuration.h"
#include "Framework/Log/Log.hpp"
#include <cstdlib>
#include <cstring>
#include <fcntl.h>
#include <linux/usb/video.h>
#include <linux/uvcvideo.h>
#include <linux/videodev2.h>
#include <poll.h>
#include <stdexcept>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <thread>
#include <unistd.h>

NaoCamera::NaoCamera(const CameraPosition cameraPosition)
  : cameraPosition_{cameraPosition}
  , mount_{(cameraPosition_ == CameraPosition::TOP) ? "topCamera" : "bottomCamera"}
{
  std::memset(&currentBuffer_, 0, sizeof(currentBuffer_));
  currentBuffer_.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
  currentBuffer_.memory = V4L2_MEMORY_MMAP;
}

NaoCamera::~NaoCamera()
{
  clearBuffers();
  close(fd_);
}

// NOLINTNEXTLINE(readability-function-cognitive-complexity)
void NaoCamera::configure(Configuration& config)
{
  config_ = &config;
  // open camera device
  const auto* const device =
      (cameraPosition_ == CameraPosition::TOP) ? "/dev/video-top" : "/dev/video-bottom";
  fd_ = open(device, O_RDWR | O_NONBLOCK);
  if (fd_ < 0)
  {
    throw std::runtime_error("Could not open camera device file!");
  }
  // mount camera configuration file
  config.mount(mount_, mount_ + ".json", ConfigurationType::HEAD);

  // image resolution
  config.get(mount_, "resolution") >> resolution_;
  if ((resolution_.x() % 16) != 0)
  {
    throw std::runtime_error(
        "The image width has to be divisible by 16 because of SSE-optimized readImage!");
  }
  // buffer registration
  config.get(mount_, "bufferCount") >> bufferCount_;

  // apply camera settings
  setFormat();
  // set desired frame rate
  setFrameRate();
  // rotate cameras
  setOrientation();
  // register buffers
  createBuffers();

  // The register 0x5001 must never be 0. It contains some bits that are for internal camera debug
  // modes only (and are set to 1 by default), so it is safe to assume that readRegister(0x5001) is
  // greater than 0
  [[maybe_unused]] std::uint16_t value = 0;
  assert(readRegister(0x5001, value) &&
         "Camera register 0x5001 contains garbage. Either camera reset "
         "was not successful or register actions are faulty");

  addV4L2CtrlSetting("autoExposure", V4L2_CID_EXPOSURE_AUTO);
  addV4L2CtrlSetting("autoWhiteBalance", V4L2_CID_AUTO_WHITE_BALANCE);
  addV4L2CtrlSetting("brightness", V4L2_CID_BRIGHTNESS);
  addV4L2CtrlSetting("contrast", V4L2_CID_CONTRAST);
  addV4L2CtrlSetting("gain", V4L2_CID_GAIN);
  addV4L2CtrlSetting("hue", V4L2_CID_HUE);
  addV4L2CtrlSetting("saturation", V4L2_CID_SATURATION);
  addV4L2CtrlSetting("sharpness", V4L2_CID_SHARPNESS);
  addV4L2CtrlSetting("whiteBalanceTemperature", V4L2_CID_WHITE_BALANCE_TEMPERATURE);
  if (config.get(mount_, "autoExposure").asInt32() != 0)
  {
    addV4L2CtrlSetting("exposure", V4L2_CID_EXPOSURE_ABSOLUTE);
  }
  addV4L2CtrlSetting("autoHue", V4L2_CID_HUE_AUTO);
  addV4L2CtrlSetting("autoFocus", V4L2_CID_FOCUS_AUTO);
  addV4L2CtrlSetting("focus", V4L2_CID_FOCUS_ABSOLUTE);

  // digital effects
  addRegisterSetting<bool>("enableDigitalEffects", [this](bool enableDigitalEffects) {
    const std::uint16_t reg = 0x5001;
    const std::uint16_t bit = 7;
    std::uint16_t registerValue = 0;
    if (!readRegister(reg, registerValue))
    {
      Log<M_TUHHSDK>(LogLevel::ERROR) << "Could not read register enableDigitalEffects 0x5001";
      assert(false);
    }
    setSingleBit(registerValue, bit, enableDigitalEffects);
    if (!writeRegister(reg, registerValue))
    {
      Log<M_TUHHSDK>(LogLevel::ERROR) << "Could not write register enableDigitalEffects 0x5001";
      assert(false);
    }
    Log<M_TUHHSDK>(LogLevel::INFO)
        << (enableDigitalEffects ? "Enabled" : "Disabled") << " digital effects";
  });

  // auto white balance bias
  addRegisterSetting<bool>("enableAWBBias", [this](bool enableAWBBias) {
    const std::uint16_t reg = 0x5005;
    const std::uint16_t bit = 5;
    std::uint16_t registerValue = 0;
    if (!readRegister(reg, registerValue))
    {
      Log<M_TUHHSDK>(LogLevel::ERROR) << "Could not read register enableAWBBias 0x5005";
      assert(false);
    }
    setSingleBit(registerValue, bit, enableAWBBias);
    if (!writeRegister(reg, registerValue))
    {
      Log<M_TUHHSDK>(LogLevel::ERROR) << "Could not write register enableAWBBias 0x5005";
      assert(false);
    }
    Log<M_TUHHSDK>(LogLevel::INFO) << (enableAWBBias ? "Enabled" : "Disabled") << " AWB bias";
  });

  // aec weights map
  addRegisterSetting<std::array<unsigned int, 16>>(
      "AECWeights", [this](std::array<unsigned int, 16> aecWeights) {
        // Set all weights at once
        std::uint8_t value[17] = {1,
                                  0,
                                  0,
                                  0,
                                  0,
                                  static_cast<unsigned char>(resolution_.x() >> 8u),
                                  static_cast<unsigned char>(resolution_.x() & 8u),
                                  static_cast<unsigned char>(resolution_.y() >> 8u),
                                  static_cast<unsigned char>(resolution_.y() & 8u)};

        for (std::size_t i = 0; i < aecWeights.size(); i += 2)
        {
          // there is only 4 bit space for a single weight
          assert(aecWeights[i] < 0x10);
          assert(aecWeights[i + 1] < 0x10);
          // the 4 left most bits are the first weight the next 4 bits are the next weight
          value[9 + i / 2] = (aecWeights[i] & 0xFu) | ((aecWeights[i + 1] & 0xFu) << 4u);
        }
        if (!setExtensionUnit(0x09, value))
        {
          Log<M_TUHHSDK>(LogLevel::ERROR) << "Failed to set AECWeights table";
          assert(false);
        }
      });

  verifyControlSettings();

  config.registerCallback(mount_, "registerAddr",
                          [this](const Uni::Value& v) { v >> registerAddr_; });
  config.registerCallback(mount_, "registerValue",
                          [this](const Uni::Value& v) { v >> registerValue_; });
  config.registerCallback(mount_, "registerWrite", [this](const Uni::Value& v) {
    v >> registerWrite_;
    onRegisterAction();
  });
}

void NaoCamera::addV4L2CtrlSetting(const std::string& name, int v4l2Command)
{
  const auto configuredValue = config_->get(mount_, name).asInt32();
  auto& setting = cameraControlSettings_.emplace_back(
      std::make_shared<V4L2CtrlSetting>(fd_, name, v4l2Command, configuredValue));
  setting->applyValue();
  config_->registerCallback(mount_, name,
                            [=](const Uni::Value& v) { setting->applyValue(v.asInt32()); });
}

void NaoCamera::startCapture()
{
  Log<M_TUHHSDK>(LogLevel::INFO) << "Starting capture for camera "
                                 << static_cast<int>(cameraPosition_);
  v4l2_buf_type type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
  if (ioctl(fd_, VIDIOC_STREAMON, &type) < 0)
  {
    throw std::runtime_error("Could not start image capturing in NaoCamera");
  }
}

void NaoCamera::stopCapture()
{
  Log<M_TUHHSDK>(LogLevel::INFO) << "Stopping capture for camera "
                                 << static_cast<int>(cameraPosition_);
  v4l2_buf_type type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
  if (ioctl(fd_, VIDIOC_STREAMOFF, &type) < 0)
  {
    throw std::runtime_error("Could not stop image capturing in NaoCamera");
  }
}

void NaoCamera::setFormat()
{
  v4l2_format fmt{};
  memset(&fmt, 0, sizeof(fmt));
  fmt.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
  fmt.fmt.pix.width = resolution_.x();
  fmt.fmt.pix.height = resolution_.y();
  fmt.fmt.pix.pixelformat = V4L2_PIX_FMT_YUYV;
  fmt.fmt.pix.field = V4L2_FIELD_NONE;
  fmt.fmt.pix.bytesperline = 2 * fmt.fmt.pix.width * fmt.fmt.pix.height;
  if (int ret = ioctl(fd_, VIDIOC_S_FMT, &fmt); ret < 0)
  {
    std::cerr << "ioctl returned: " << std::to_string(ret) << " errno: " << errno << '\n';
    throw std::runtime_error("Could not set image format in NaoCamera");
  }
  if ((fmt.type != V4L2_BUF_TYPE_VIDEO_CAPTURE) ||
      (fmt.fmt.pix.width != static_cast<__u32>(resolution_.x())) ||
      (fmt.fmt.pix.height != static_cast<__u32>(resolution_.y())) ||
      (fmt.fmt.pix.pixelformat != V4L2_PIX_FMT_YUYV) || (fmt.fmt.pix.field != V4L2_FIELD_NONE))
  {
    throw std::runtime_error(
        "Could set image format but the driver does not accept the settings in NaoCamera");
  }
}

void NaoCamera::setFrameRate() const
{
  v4l2_streamparm streamParam{};
  memset(&streamParam, 0, sizeof(streamParam));
  streamParam.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;

  if (ioctl(fd_, VIDIOC_G_PARM, &streamParam) != 0)
  {
    throw std::runtime_error("Could not read frame rate in NaoCamera");
  }
  // framerate
  unsigned int fps{static_cast<unsigned int>(config_->get(mount_, "fps").asInt32())};

  streamParam.parm.capture.timeperframe.numerator = 1;
  streamParam.parm.capture.timeperframe.denominator = fps;

  if (ioctl(fd_, VIDIOC_S_PARM, &streamParam) < 0)
  {
    throw std::runtime_error("Could not set frame rate in NaoCamera");
  }
  if ((streamParam.type != V4L2_BUF_TYPE_VIDEO_CAPTURE) ||
      (streamParam.parm.capture.timeperframe.numerator != 1) ||
      (streamParam.parm.capture.timeperframe.denominator != fps))
  {
    throw std::runtime_error(
        "Could set frame rate but the driver does not accept the settings in NaoCamera");
  }
}

void NaoCamera::setOrientation()
{
  std::uint16_t value = cameraPosition_ == CameraPosition::TOP ? 1 : 0;
  // horizontal flip
  const bool successfulHorizontal = setExtensionUnit(0x0c, value);
  if (!successfulHorizontal)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR) << "Unable to set orientation (horizontal)";
  }
  assert(successfulHorizontal);
  // vertical flip
  const bool successfulVertical = setExtensionUnit(0x0d, value);
  if (!successfulVertical)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR) << "Unable to set orientation (vertical)";
  }
  assert(successfulVertical);
}

void NaoCamera::createBuffers()
{
  v4l2_buffer buf{};
  v4l2_requestbuffers reqbufs{};

  bufferMem_.resize(bufferCount_);
  bufferLength_.resize(bufferCount_);

  memset(&reqbufs, 0, sizeof(reqbufs));
  reqbufs.count = bufferCount_;
  reqbufs.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
  reqbufs.memory = V4L2_MEMORY_MMAP;
  if (ioctl(fd_, VIDIOC_REQBUFS, &reqbufs) < 0)
  {
    throw std::runtime_error("Could not request buffers from driver in NaoCamera");
  }
  for (unsigned int i = 0; i < bufferCount_; i++)
  {
    memset(&buf, 0, sizeof(buf));
    buf.index = i;
    buf.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
    if (ioctl(fd_, VIDIOC_QUERYBUF, &buf) < 0)
    {
      for (int j = i - 1; j >= 0; --j)
      {
        munmap(bufferMem_[j], bufferLength_[j]);
      }
      throw std::runtime_error("Could not get buffer in NaoCamera");
    }
    bufferLength_[i] = buf.length;
    bufferMem_[i] = static_cast<unsigned char*>(
        mmap(nullptr, bufferLength_[i], PROT_READ | PROT_WRITE, MAP_SHARED, fd_, buf.m.offset));
    if (bufferMem_[i] == MAP_FAILED)
    {
      for (int j = i - 1; j >= 0; --j)
      {
        munmap(bufferMem_[j], bufferLength_[j]);
      }
      throw std::runtime_error("Could not map buffer in NaoCamera");
    }
    if (ioctl(fd_, VIDIOC_QBUF, &buf) < 0)
    {
      for (int j = i - 1; j >= 0; --j)
      {
        munmap(bufferMem_[j], bufferLength_[j]);
      }
      throw std::runtime_error("Could not enqueue buffer in NaoCamera");
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
    if (bufferMem_[i] != nullptr)
    {
      munmap(bufferMem_[i], bufferLength_[i]);
      bufferMem_[i] = nullptr;
    }
  }
  bufferMem_.clear();
  bufferLength_.clear();

  buffersInitialized_ = false;
}

void NaoCamera::verifyControlSettings()
{
  for (const auto& setting : cameraControlSettings_)
  {
    if (!setting->isAppliedGracefully())
    {
      Log<M_TUHHSDK>(LogLevel::ERROR)
          << "Setting \"" << setting->getName() << "\" altered from configured value";
      assert(false);
    }
  }
}

void NaoCamera::onRegisterAction()
{
  if (registerWrite_)
  {
    Log<M_TUHHSDK>(LogLevel::INFO) << "WRITE_REGISTER " << std::to_string(registerAddr_)
                                   << " VALUE = " << std::to_string(registerValue_);
    if (!writeRegister(static_cast<std::uint16_t>(registerAddr_),
                       static_cast<std::uint16_t>(registerValue_)))
    {
      Log<M_TUHHSDK>(LogLevel::ERROR) << "Failed to write register at address " << registerAddr_
                                      << " with value " << registerValue_;
    }
  }
  else
  {
    Log<M_TUHHSDK>(LogLevel::INFO) << "READ_REGISTER ADDR: " << std::to_string(registerAddr_);
    std::uint16_t newValue = 0;
    if (!readRegister(static_cast<std::uint16_t>(registerAddr_), newValue))
    {
      Log<M_TUHHSDK>(LogLevel::ERROR) << "Failed to read register at address " << registerAddr_;
    }
    config_->set(mount_, "registerValue", Uni::Value(static_cast<int32_t>(newValue)));
  }
}

bool NaoCamera::queryExtensionUnit(bool set, __u8 selector, __u8* data, __u16 size) const
{
  uvc_xu_control_query xu{};
  xu.unit = 3;
  xu.selector = selector;
  xu.query = set ? UVC_SET_CUR : UVC_GET_CUR;
  xu.size = size;
  xu.data = data;
  if (ioctl(fd_, UVCIOC_CTRL_QUERY, &xu) != 0)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR) << "Query Extension Unit (selector: "
                                    << selector
                                    // NOLINTNEXTLINE(concurrency-mt-unsafe)
                                    << ") failed with errno: " << std::strerror(errno);
    return false;
  }
  return true;
}

bool NaoCamera::waitForCameras(std::array<NaoCamera*, 2> cameras, int timeout)
{
  std::array<pollfd, cameras.size()> pollfds{};
  for (std::size_t i = 0; i < cameras.size(); ++i)
  {
    // Only poll cameras without valid image
    int fd = cameras[i]->imageValid_ ? -1 : cameras[i]->fd_;
    pollfds[i] = {fd, POLLIN | POLLPRI, 0};
  }

  int polled = poll(pollfds.data(), cameras.size(), timeout);
  if (polled < 0)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR) << "Unable to fetch images. Image poll returned -1 (Error)";
    assert(false);
    return false;
  }
  if (polled == 0)
  {
    Log<M_TUHHSDK>(LogLevel::ERROR)
        << "Unable to fetch images. Image poll returned 0 (poll timed out)";
    return false;
  }

  for (std::size_t i = 0; i < cameras.size(); ++i)
  {
    if ((pollfds[i].revents & POLLIN) != 0)
    {
      v4l2_buffer lastBuffer{};
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
          Log<M_TUHHSDK>(LogLevel::WARNING) << "Dropped a frame";
        }
        lastBuffer = cameras[i]->currentBuffer_;
      }

      // errno is EAGAIN if the nonblocking VIDIOC_DQBUF returned without an image availabe.
      // So after removing all waiting images from the queue the queue should be empty
      // and thus the errno should be EAGAIN
      if (errno != EAGAIN)
      {
        Log<M_TUHHSDK>(LogLevel::ERROR) << "VIDEOC_DQBUF is != EAGAIN. No image available";
        return false;
      }
      // V4L2 gives the time at which the first pixel of the image was recorded as timeval
      // "+ i * 1000": This is a hack. When top and bottom camera do have the same timestamp
      //               one of them will be skipped in our current debug protocol impl.
      cameras[i]->timePoint_ = Clock::time_point{std::chrono::microseconds{
          static_cast<__u64>(cameras[i]->currentBuffer_.timestamp.tv_sec) * 1000000LL +
          cameras[i]->currentBuffer_.timestamp.tv_usec + i * 1000}};
      // This fix is needed as the first image that we get on the v6 hardware has a timestamp
      // that does not make any sense (to @rkost, @nagua).
      cameras[i]->imageValid_ = cameras[i]->timePoint_ >= Clock::time_point();
      if (!cameras[i]->imageValid_)
      {
        Log<M_TUHHSDK>(LogLevel::WARNING)
            << "Camera timestamp smaller than base time (normal during the "
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
    else if (pollfds[i].revents != 0)
    {
      Log<M_TUHHSDK>(LogLevel::ERROR) << "Camera is in an unknown state (This is really bad).";
      assert(false && "Strange camera error perhaps add automatic camera resetting");
      return false;
    }
  }
  return true;
}

void NaoCamera::produce(CycleInfo& cycleInfo, ImageData& imageData)
{
  imageData.image422.setData(reinterpret_cast<YCbCr422*>(bufferMem_[currentBuffer_.index]),
                             resolution_);
  imageData.cameraPosition = cameraPosition_;
  imageData.identification = imageData.cameraPosition == CameraPosition::TOP ? "top" : "bottom";
  imageData.captureTimePoint = timePoint_;
  cycleInfo.startTime = timePoint_;
}

void NaoCamera::releaseImage()
{
  if (imageValid_)
  {
    if (ioctl(fd_, VIDIOC_QBUF, &currentBuffer_) < 0)
    {
      throw std::runtime_error("Unable to queue buffer");
    }
    imageValid_ = false;
  }
}

CameraPosition NaoCamera::getCameraPosition()
{
  return cameraPosition_;
}

void NaoCamera::setSingleBit(std::uint16_t& value, std::uint8_t bit, bool enable)
{
  // This sets the nth bit of value to "enable"
  // See: https://stackoverflow.com/questions/47981/how-do-you-set-clear-and-toggle-a-single-bit
  value = value ^ ((-static_cast<uint16_t>(enable ? 1 : 0) ^ value) & (1u << bit));
}

bool NaoCamera::readRegister(std::uint16_t address, std::uint16_t& value) const
{

  std::uint8_t bytes[5] = {0, static_cast<unsigned char>(address >> 8u),
                           static_cast<unsigned char>(address & 0xFFu)};
  if (setExtensionUnit(0x0e, bytes))
  {
    std::this_thread::sleep_for(std::chrono::milliseconds(10));
    if (getExtensionUnit(14, bytes))
    {
      value = static_cast<std::uint16_t>(bytes[3] << 8u | bytes[4]);
      return true;
    }
  }
  return false;
}

bool NaoCamera::writeRegister(std::uint16_t address, std::uint16_t value) const
{
  std::uint8_t bytes[5];
  bytes[0] = 1;
  bytes[1] = static_cast<unsigned char>(address >> 8u);
  bytes[2] = static_cast<unsigned char>(address & 0xFFu);
  bytes[3] = static_cast<unsigned char>(value >> 8u);
  bytes[4] = static_cast<unsigned char>(value & 0xFFu);
  return setExtensionUnit(0x0e, bytes);
}

template <typename T>
void NaoCamera::addRegisterSetting(const std::string& key, std::function<void(T)> callback) const
{
  T value;
  config_->get(mount_, key) >> value;
  callback(value);
  config_->registerCallback(mount_, key, [callback, key](const Uni::Value& v) {
    T value;
    v >> value;
    callback(value);
  });
}
