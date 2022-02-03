#include "Hardware/Webots/WebotsCamera.hpp"
#include "Framework/Log/Log.hpp"
#include <memory>

WebotsCamera::WebotsCamera(webots::Camera* camera, CameraPosition cameraPosition)
  : camera_{camera}
  , cameraPosition_{cameraPosition}
{
  assert(camera_ != nullptr);
  Log<M_TUHHSDK>{LogLevel::INFO} << (cameraPosition_ == CameraPosition::TOP ? "Top" : "Bottom")
                                 << " Camera: width=" << camera_->getWidth()
                                 << ", height=" << camera_->getHeight();
}

void WebotsCamera::updateImage(const Clock::time_point& timePoint)
{
  {
    std::lock_guard lock{imageMutex_};
    image_.resize({camera_->getWidth(), camera_->getHeight()});
    const auto* imageDataInput{camera_->getImage()};

    if (imageDataInput != nullptr)
    {
      YCbCr422* imageDataOutput{image_.data};
      // TODO: SSE/AVX(2) optimization
      for (int y{0}; y < image_.size.y(); y++)
      {
        for (int x{0}; x < image_.size.x(); x++, imageDataOutput++)
        {
          // Webots format is BGRA
          const auto blue1{*(imageDataInput++)};
          const auto green1{*(imageDataInput++)};
          const auto red1{*(imageDataInput++)};
          ++imageDataInput; // alpha channel
          const auto blue2{*(imageDataInput++)};
          const auto green2{*(imageDataInput++)};
          const auto red2{*(imageDataInput++)};
          ++imageDataInput; // alpha channel

          // YCbCr to RGB conversion
          // Conversion factors from
          // https://de.wikipedia.org/wiki/YCbCr-Farbmodell#Umrechnung_zwischen_RGB_und_YCbCr
          imageDataOutput->y1 = static_cast<std::uint8_t>(
              std::clamp(0.299f * static_cast<float>(red1) + 0.587f * static_cast<float>(green1) +
                             0.114f * static_cast<float>(blue1),
                         0.f, 255.f));
          imageDataOutput->cb = static_cast<std::uint8_t>(std::clamp(
              128.f - 0.168736f * static_cast<float>(red1) -
                  0.331264f * static_cast<float>(green1) + 0.5f * static_cast<float>(blue1),
              0.f, 255.f));
          imageDataOutput->y2 = static_cast<std::uint8_t>(
              std::clamp(0.299f * static_cast<float>(red2) + 0.587f * static_cast<float>(green2) +
                             0.114f * static_cast<float>(blue2),
                         0.f, 255.f));
          imageDataOutput->cr = static_cast<std::uint8_t>(std::clamp(
              128.f + 0.5f * static_cast<float>(red1) - 0.418688f * static_cast<float>(green1) -
                  0.081312f * static_cast<float>(blue1),
              0.f, 255.f));
        }
      }
    }

    timePoint_ = timePoint;

    imageUpdated_ = true;
  }
  imageUpdatedConditionVariable_.notify_all();
}

void WebotsCamera::produce(CycleInfo& cycleInfo, ImageData& imageData)
{
  std::unique_lock lock{imageMutex_};
  imageUpdatedConditionVariable_.wait(lock, [this] { return imageUpdated_; });
  imageUpdated_ = false;

  imageData.image422 = image_;
  imageData.cameraPosition = cameraPosition_;
  imageData.identification = imageData.cameraPosition == CameraPosition::TOP ? "top" : "bottom";
  imageData.captureTimePoint = timePoint_;
  cycleInfo.startTime = timePoint_;
}

void WebotsCamera::enable()
{
  constexpr auto samplingPeriodInMilliseconds{1000 / 30};
  camera_->enable(samplingPeriodInMilliseconds);
}

void WebotsCamera::disable()
{
  camera_->disable();
}

CameraPosition WebotsCamera::getCameraPosition()
{
  return cameraPosition_;
}
