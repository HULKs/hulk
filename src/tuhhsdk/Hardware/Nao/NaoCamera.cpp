#include <cstdlib>
#include <cstring>
#include <stdexcept>
#include <sys/fcntl.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <unistd.h>
#include <x86intrin.h>

#include "Modules/Configuration/Configuration.h"
#include "print.h"

#include "NaoCamera.hpp"
#include "NaoCameraCommon.hpp"


char NaoCamera::shuffle1[16] = {0, 1, 3, 2, 1, 3, 4, 5, 7, 6, 5, 7, 8, 9, 11, 10};
char NaoCamera::shuffle2[16] = {1, 3, 4, 5, 7, 6, 5, 7, 8, 9, 11, 10, 9, 11, 12, 13};
char NaoCamera::shuffle3[16] = {7, 6, 5, 7, 8, 9, 11, 10, 9, 11, 12, 13, 15, 14, 13, 15};

NaoCamera::NaoCamera(const Camera camera)
  : camera_(camera)
  , mount_((camera_ == Camera::TOP) ? "topCamera" : "bottomCamera")
  , fd_(-1)
  , bufferCount_(0)
  , bufferMem_(NULL)
  , bufferLength_(NULL)
{
  const char* device = (camera_ == Camera::TOP) ? "/dev/video0" : "/dev/video1";
  fd_ = open(device, O_RDWR);
  if (fd_ < 0)
  {
    throw std::runtime_error("Could not open camera device file!");
  }
}

NaoCamera::~NaoCamera()
{
  if (bufferMem_)
  {
    if (bufferLength_)
    {
      for (unsigned int i = 0; i < bufferCount_; i++)
      {
        if (bufferLength_[i])
        {
          munmap(bufferMem_[i], bufferLength_[i]);
        }
      }
      free(bufferLength_);
    }
    free(bufferMem_);
  }
  close(fd_);
}

void NaoCamera::configure(Configuration& config)
{
  config.mount(mount_, mount_ + ".json", ConfigurationType::HEAD);

  config.get(mount_, "resolution") >> resolution_;
  config.get(mount_, "fps") >> fps_;
  config.get(mount_, "bufferCount") >> bufferCount_;
  config.get(mount_, "exposure") >> exposure_;
  config.get(mount_, "gain") >> gain_;
  config.get(mount_, "whiteBalanceTemperature") >> whiteBalanceTemperature_;
  config.get(mount_, "contrast") >> contrast_;
  config.get(mount_, "gamma") >> gamma_;
  config.get(mount_, "hue") >> hue_;
  config.get(mount_, "saturation") >> saturation_;
  config.get(mount_, "sharpness") >> sharpness_;
  config.get(mount_, "fadeToBlack") >> fadeToBlack_;

  config.get(mount_, "brightness") >> brightness_;
  config.get(mount_, "brightnessDark") >> brightnessDark_;
  config.get(mount_, "exposureAlgorithm") >> exposureAlgorithm_;
  config.get(mount_, "aeTargetGain") >> aeTargetGain_;
  config.get(mount_, "aeMinAGain") >> aeMinAGain_;
  config.get(mount_, "aeMaxAGain") >> aeMaxAGain_;
  config.get(mount_, "aeMinDGain") >> aeMinDGain_;
  config.get(mount_, "aeMaxDGain") >> aeMaxDGain_;

  if ((resolution_.x() % 16) != 0)
  {
    throw std::runtime_error("The image width has to be divisible by 16 because of SSE-optimized readImage!");
  }

  setFormat();
  setFrameRate();
  setControlSettings();
  createBuffers();

  config.registerCallback(mount_, "exposure", [this](const Uni::Value& v) {
    v >> exposure_;
    onExposureChange();
  });
  config.registerCallback(mount_, "exposureAlgorithm", [this](const Uni::Value& v) {
    v >> exposureAlgorithm_;
    onExposureChange();
  });
  config.registerCallback(mount_, "aeTargetGain", [this](const Uni::Value& v) {
    v >> aeTargetGain_;
    onExposureChange();
  });
  config.registerCallback(mount_, "aeMinAGain", [this](const Uni::Value& v) {
    v >> aeMinAGain_;
    onExposureChange();
  });
  config.registerCallback(mount_, "aeMaxAGain", [this](const Uni::Value& v) {
    v >> aeMaxAGain_;
    onExposureChange();
  });
  config.registerCallback(mount_, "aeMinDGain", [this](const Uni::Value& v) {
    v >> aeMinDGain_;
    onExposureChange();
  });
  config.registerCallback(mount_, "aeMaxDGain", [this](const Uni::Value& v) {
    v >> aeMaxDGain_;
    onExposureChange();
  });

  config.registerCallback(mount_, "gain", [this](const Uni::Value& v) {
    v >> gain_;
    onGainChange();
  });
  config.registerCallback(mount_, "brightness", [this](const Uni::Value& v) {
    v >> brightness_;
    onGainChange();
  });
  config.registerCallback(mount_, "brightnessDark", [this](const Uni::Value& v) {
    v >> brightnessDark_;
    onGainChange();
  });

  config.registerCallback(mount_, "whiteBalanceTemperature", boost::bind(&NaoCamera::onWhiteBalanceTemperatureChange, this, _1));
  config.registerCallback(mount_, "contrast", boost::bind(&NaoCamera::onContrastChange, this, _1));
  config.registerCallback(mount_, "gamma", boost::bind(&NaoCamera::onGammaChange, this, _1));
  config.registerCallback(mount_, "hue", boost::bind(&NaoCamera::onHueChange, this, _1));
  config.registerCallback(mount_, "saturation", boost::bind(&NaoCamera::onSaturationChange, this, _1));
  config.registerCallback(mount_, "sharpness", boost::bind(&NaoCamera::onSharpnessChange, this, _1));
  config.registerCallback(mount_, "fadeToBlack", boost::bind(&NaoCamera::onFadeToBlackChange, this, _1));
}

float NaoCamera::waitForImage()
{
  fd_set fds;
  timeval timeout;
  FD_ZERO(&fds);
  FD_SET(fd_, &fds);
  timeout.tv_sec = 0;
  timeout.tv_usec = 1000000 / fps_;
  if (select(fd_ + 1, &fds, NULL, NULL, &timeout) < 0)
  {
    throw std::runtime_error("select error in NaoCamera!");
  }
  // Timeout now contains the remaining time of the original timeout.
  return (1000000 / fps_ - timeout.tv_usec) / 1000000.f;
}

TimePoint NaoCamera::readImage(Image& image)
{
  unsigned char* src;
  Color* dst;
  v4l2_buffer buf;
  memset(&buf, 0, sizeof(buf));
  buf.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
  buf.memory = V4L2_MEMORY_MMAP;
  if (ioctl(fd_, VIDIOC_DQBUF, &buf) < 0)
  {
    throw std::runtime_error("DQBUF error in NaoCamera! Maybe there is already a program using the camera on the NAO?");
  }
  if (buf.index >= bufferCount_)
  {
    throw std::runtime_error("Buffer index greater or equal than the number of buffers in NaoCamera!");
  }
  image.resize(resolution_);
  // Convert the YUYV image to a YUV image by duplicating the U and V channel.
  __m128i shuffle1mm = _mm_loadu_si128(reinterpret_cast<__m128i*>(shuffle1));
  __m128i shuffle2mm = _mm_loadu_si128(reinterpret_cast<__m128i*>(shuffle2));
  __m128i shuffle3mm = _mm_loadu_si128(reinterpret_cast<__m128i*>(shuffle3));
  src = bufferMem_[buf.index];
  dst = image.data_;
  unsigned char* end = src + 2 * resolution_.x() * resolution_.y();
  for (; src < end; dst += 16, src += 32)
  {
    __m128i yuvpixels1 = _mm_loadu_si128(reinterpret_cast<__m128i*>(src));
    __m128i yuyvpixels1 = _mm_shuffle_epi8(yuvpixels1, shuffle1mm);

    __m128i yuvpixels1point5 = _mm_loadu_si128(reinterpret_cast<__m128i*>(src + 8));
    __m128i yuyvpixels2 = _mm_shuffle_epi8(yuvpixels1point5, shuffle2mm);

    __m128i yuvpixels2 = _mm_loadu_si128(reinterpret_cast<__m128i*>(src + 16));
    __m128i yuyvpixels3 = _mm_shuffle_epi8(yuvpixels2, shuffle3mm);

    _mm_storeu_si128(reinterpret_cast<__m128i*>(dst), yuyvpixels1);
    _mm_storeu_si128(reinterpret_cast<__m128i*>(dst) + 1, yuyvpixels2);
    _mm_storeu_si128(reinterpret_cast<__m128i*>(dst) + 2, yuyvpixels3);
  }
  if (ioctl(fd_, VIDIOC_QBUF, &buf) < 0)
  {
    throw std::runtime_error("QBUF error in NaoCamera!");
  }
  // V4L2 gives the time at which the first pixel of the image was recorded as timeval
  const unsigned int millisecondsSince1970 = buf.timestamp.tv_sec * 1000 + buf.timestamp.tv_usec / 1000;
  return TimePoint(millisecondsSince1970 - TimePoint::getBaseTime());
}

void NaoCamera::startCapture()
{
  v4l2_buf_type type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
  if (ioctl(fd_, VIDIOC_STREAMON, &type) < 0)
  {
    throw std::runtime_error("Could not start image capturing in NaoCamera!");
  }
}

void NaoCamera::stopCapture()
{
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
  if ((fmt.type != V4L2_BUF_TYPE_VIDEO_CAPTURE) || (fmt.fmt.pix.width != (__u32)resolution_.x()) || (fmt.fmt.pix.height != (__u32)resolution_.y()) ||
      (fmt.fmt.pix.pixelformat != V4L2_PIX_FMT_YUYV) || (fmt.fmt.pix.field != V4L2_FIELD_NONE))
  {
    throw std::runtime_error("Could set image format but the driver does not accept the settings in NaoCamera!");
  }
}

void NaoCamera::setFrameRate()
{
  v4l2_streamparm fps;
  memset(&fps, 0, sizeof(fps));
  fps.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
  fps.parm.capture.timeperframe.numerator = 1;
  fps.parm.capture.timeperframe.denominator = fps_;
  if (ioctl(fd_, VIDIOC_S_PARM, &fps) < 0)
  {
    throw std::runtime_error("Could not set frame rate in NaoCamera!");
  }
  if ((fps.type != V4L2_BUF_TYPE_VIDEO_CAPTURE) || (fps.parm.capture.timeperframe.numerator != 1) || (fps.parm.capture.timeperframe.denominator != fps_))
  {
    throw std::runtime_error("Could set frame rate but the driver does not accept the settings in NaoCamera!");
  }
}

void NaoCamera::setControlSettings()
{
  if (exposure_)
  {
    setControlSetting(V4L2_CID_EXPOSURE_AUTO, 0);
    setControlSetting(V4L2_CID_EXPOSURE, exposure_);
    setControlSetting(V4L2_CID_GAIN, gain_);
  }
  else
  {
    setControlSetting(V4L2_CID_EXPOSURE_AUTO, 1);
    setControlSetting(V4L2_CID_BRIGHTNESS, brightness_);
    setControlSetting(V4L2_MT9M114_BRIGHTNESS_DARK, brightnessDark_);
    setControlSetting(V4L2_CID_EXPOSURE_ALGORITHM, exposureAlgorithm_);
    setControlSetting(V4L2_MT9M114_AE_TARGET_GAIN, aeTargetGain_);

    setControlSetting(V4L2_MT9M114_AE_MIN_VIRT_AGAIN, aeMinAGain_);
    setControlSetting(V4L2_MT9M114_AE_MAX_VIRT_AGAIN, aeMaxAGain_);
    setControlSetting(V4L2_MT9M114_AE_MIN_VIRT_DGAIN, aeMinDGain_);
    setControlSetting(V4L2_MT9M114_AE_MAX_VIRT_DGAIN, aeMaxDGain_);
  }

  setControlSetting(V4L2_CID_CONTRAST, contrast_);
  setControlSetting(V4L2_CID_GAMMA, gamma_);
  setControlSetting(V4L2_CID_HUE, hue_);
  setControlSetting(V4L2_CID_SATURATION, saturation_);
  setControlSetting(V4L2_CID_SHARPNESS, sharpness_);
  setControlSetting(V4L2_MT9M114_FADE_TO_BLACK, fadeToBlack_);
  setControlSetting(V4L2_CID_POWER_LINE_FREQUENCY, V4L2_CID_POWER_LINE_FREQUENCY_50HZ);

  switch (camera_)
  {
    case Camera::TOP:
      setControlSetting(V4L2_CID_HFLIP, 1);
      setControlSetting(V4L2_CID_VFLIP, 1);
      break;
    case Camera::BOTTOM:
    default:
      setControlSetting(V4L2_CID_HFLIP, 0);
      setControlSetting(V4L2_CID_VFLIP, 0);
      break;
  }

  if (whiteBalanceTemperature_)
  {
    setControlSetting(V4L2_CID_AUTO_WHITE_BALANCE, 0);
    setControlSetting(V4L2_CID_WHITE_BALANCE_TEMPERATURE, whiteBalanceTemperature_);
  }
  else
  {
    setControlSetting(V4L2_CID_AUTO_WHITE_BALANCE, 1);
  }
}

void NaoCamera::createBuffers()
{
  v4l2_buffer buf;
  v4l2_requestbuffers reqbufs;
  if (bufferMem_ || bufferLength_)
  {
    throw std::runtime_error("TODO");
  }
  bufferMem_ = (unsigned char**)calloc(bufferCount_, sizeof(unsigned char*));
  if (!bufferMem_)
  {
    throw std::runtime_error("Could not get memory for buffer memory in NaoCamera!");
  }
  bufferLength_ = (unsigned int*)calloc(bufferCount_, sizeof(unsigned int));
  if (!bufferLength_)
  {
    free(bufferMem_);
    throw std::runtime_error("Could not get memory for buffer length in NaoCamera!");
  }
  memset(&reqbufs, 0, sizeof(reqbufs));
  reqbufs.count = bufferCount_;
  reqbufs.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
  reqbufs.memory = V4L2_MEMORY_MMAP;
  if (ioctl(fd_, VIDIOC_REQBUFS, &reqbufs) < 0)
  {
    free(bufferLength_);
    free(bufferMem_);
    throw std::runtime_error("Could not request buffers from driver in NaoCamera!");
  }
  for (unsigned int i = 0; i < bufferCount_; i++)
  {
    memset(&buf, 0, sizeof(buf));
    buf.index = i;
    buf.type = V4L2_BUF_TYPE_VIDEO_CAPTURE;
    if (ioctl(fd_, VIDIOC_QUERYBUF, &buf) < 0)
    {
      for (--i; i < bufferCount_; i--)
      {
        munmap(bufferMem_[i], bufferLength_[i]);
      }
      free(bufferLength_);
      free(bufferMem_);
      throw std::runtime_error("Could not get buffer in NaoCamera!");
    }
    bufferLength_[i] = buf.length;
    bufferMem_[i] = (unsigned char*)mmap(0, bufferLength_[i], PROT_READ | PROT_WRITE, MAP_SHARED, fd_, buf.m.offset);
    if (bufferMem_[i] == MAP_FAILED)
    {
      for (--i; i < bufferCount_; i--)
      {
        munmap(bufferMem_[i], bufferLength_[i]);
      }
      free(bufferLength_);
      free(bufferMem_);
      throw std::runtime_error("Could not map buffer in NaoCamera!");
    }
    if (ioctl(fd_, VIDIOC_QBUF, &buf) < 0)
    {
      for (; i < bufferCount_; i--)
      {
        munmap(bufferMem_[i], bufferLength_[i]);
      }
      free(bufferLength_);
      free(bufferMem_);
      throw std::runtime_error("Could not enqueue buffer in NaoCamera!");
    }
  }
}

void NaoCamera::setControlSetting(__u32 id, __s32 value)
{
  v4l2_control ctrl;
  v4l2_queryctrl qctrl;
  for (unsigned int i = 0; i < CONTROL_SETTING_TRIES; i++)
  {
    memset(&ctrl, 0, sizeof(ctrl));
    memset(&qctrl, 0, sizeof(qctrl));
    qctrl.id = id;
    if (ioctl(fd_, VIDIOC_QUERYCTRL, &qctrl) < 0)
    {
      continue;
    }
    if (qctrl.flags & V4L2_CTRL_FLAG_DISABLED)
    {
      continue;
    }
    // crop values if necessary
    if (value < qctrl.minimum)
    {
      value = qctrl.minimum;
    }
    else if (value > qctrl.maximum)
    {
      value = qctrl.maximum;
    }
    ctrl.id = id;
    ctrl.value = value;
    if (ioctl(fd_, VIDIOC_S_CTRL, &ctrl) < 0)
    {
      continue;
    }
    return;
  }
  // This is bad, but throwing an exception would be bad, too.
  Log(LogLevel::ERROR) << "A camera control setting: " << (int)id << " could not be set.";
}

void NaoCamera::onExposureChange()
{
  if (exposure_)
  {
    setControlSetting(V4L2_CID_EXPOSURE_AUTO, 0);
    setControlSetting(V4L2_CID_EXPOSURE, exposure_);
    setControlSetting(V4L2_CID_GAIN, gain_);
  }
  else
  {
    setControlSetting(V4L2_CID_EXPOSURE_AUTO, 1);
    setControlSetting(V4L2_CID_BRIGHTNESS, brightness_);
    setControlSetting(V4L2_MT9M114_BRIGHTNESS_DARK, brightnessDark_);
    setControlSetting(V4L2_CID_EXPOSURE_ALGORITHM, exposureAlgorithm_);
    setControlSetting(V4L2_MT9M114_AE_TARGET_GAIN, aeTargetGain_);

    setControlSetting(V4L2_MT9M114_AE_MIN_VIRT_AGAIN, aeMinAGain_);
    setControlSetting(V4L2_MT9M114_AE_MAX_VIRT_AGAIN, aeMaxAGain_);
    setControlSetting(V4L2_MT9M114_AE_MIN_VIRT_DGAIN, aeMinDGain_);
    setControlSetting(V4L2_MT9M114_AE_MAX_VIRT_DGAIN, aeMaxDGain_);
  }
}

void NaoCamera::onGainChange()
{
  if (exposure_)
  {
    setControlSetting(V4L2_CID_GAIN, gain_);
  }
  else
  {
    setControlSetting(V4L2_CID_BRIGHTNESS, brightness_);
    setControlSetting(V4L2_MT9M114_BRIGHTNESS_DARK, brightnessDark_);
  }
}

void NaoCamera::onWhiteBalanceTemperatureChange(const Uni::Value& value)
{
  value >> whiteBalanceTemperature_;
  if (whiteBalanceTemperature_)
  {
    setControlSetting(V4L2_CID_AUTO_WHITE_BALANCE, 0);
    setControlSetting(V4L2_CID_WHITE_BALANCE_TEMPERATURE, whiteBalanceTemperature_);
  }
  else
  {
    setControlSetting(V4L2_CID_AUTO_WHITE_BALANCE, 1);
  }
}

void NaoCamera::onContrastChange(const Uni::Value& value)
{
  value >> contrast_;
  setControlSetting(V4L2_CID_CONTRAST, contrast_);
}

void NaoCamera::onGammaChange(const Uni::Value& value)
{
  value >> gamma_;
  setControlSetting(V4L2_CID_GAMMA, gamma_);
}

void NaoCamera::onHueChange(const Uni::Value& value)
{
  value >> hue_;
  setControlSetting(V4L2_CID_HUE, hue_);
}

void NaoCamera::onSaturationChange(const Uni::Value& value)
{
  value >> saturation_;
  setControlSetting(V4L2_CID_SATURATION, saturation_);
}

void NaoCamera::onSharpnessChange(const Uni::Value& value)
{
  value >> sharpness_;
  setControlSetting(V4L2_CID_SHARPNESS, sharpness_);
}

void NaoCamera::onFadeToBlackChange(const Uni::Value& value)
{
  value >> fadeToBlack_;
  setControlSetting(V4L2_MT9M114_FADE_TO_BLACK, fadeToBlack_);
}
