#include "Tools/Chronometer.hpp"
#include "Tools/Storage/Image.hpp"

#include "OneMeansFieldColorDetection.hpp"

#include <limits>

using OMFCD = OneMeansFieldColorDetection;

OMFCD::OneMeansFieldColorDetection(const ModuleManagerInterface& manager)
  : Module(manager, "OneMeansFieldColorDetection")
  , calculateInitialGuess_(*this, "calculateInitialGuess",
                           [this] {
                             updateInitialGuessBottom_ = true;
                             updateInitialGuessTop_ = true;
                           })
  , initialGuessTop_(*this, "initialGuessTop", [] {})
  , initialGuessBottom_(*this, "initialGuessBottom", [] {})
  , thresholdY_(*this, "thresholdY", [] {})
  , thresholdUV_(*this, "thresholdUV", [] {})
  , sampleRate_(10)
  , imageData_(*this)
  , cameraMatrix_(*this)
  , fieldColor_(*this)
  , updateInitialGuessTop_(false)
  , updateInitialGuessBottom_(false)
{
}

void OMFCD::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");
  const Image& image = imageData_->image;

  horizonY_ = cameraMatrix_->getHorizonHeight();
  if (horizonY_ >= image.size_.y())
  {
    // The ground is not visible at the moment.
    sendImageForDebug(image);
    return;
  }
  Vector2f initialGuess;
  if (imageData_->camera == Camera::TOP)
  {
    if (updateInitialGuessTop_)
    {
      Uni::Value value;
      value << initialStep(imageData_->image, 200, horizonY_);
      configuration().set(mount_, "initialGuessTop", value);
      updateInitialGuessTop_ = false;
    }
    initialGuess = initialGuessTop_();
  }
  else
  {
    if (updateInitialGuessBottom_)
    {
      Uni::Value value;
      value << initialStep(imageData_->image, 200, horizonY_);
      configuration().set(mount_, "initialGuessBottom", value);
      updateInitialGuessBottom_ = false;
    }
    initialGuess = initialGuessBottom_();
  }

  const int thresholdUVSquared = thresholdUV_() * thresholdUV_();
  const FieldColorCluster initialColor = {initialGuess, 200};

  FieldColorCluster color = initialColor;
  const int iterCount = 3;
  for (int i = 0; i < iterCount; i++)
  {
    const FieldColorCluster newColor = updateStep(image, color, thresholdUVSquared, horizonY_);
    if ((initialColor.mean - newColor.mean).squaredNorm() > thresholdUVSquared)
    {
      color = initialColor;
      break;
    }
    color = newColor;
  }
  fieldColor_->thresholdUvSquared = thresholdUVSquared;
  fieldColor_->thresholdY = color.yThresh;
  fieldColor_->meanCb = static_cast<int>(color.mean.x());
  fieldColor_->meanCr = static_cast<int>(color.mean.y());
  fieldColor_->valid = true;

  sendImageForDebug(image);
}

Vector2f OMFCD::initialStep(const Image& image, const int yThresh, const int startY) const
{
  std::array<int, 256> histCb = {{}};
  std::array<int, 256> histCr = {{}};
  for (int y = startY; y < image.size_.y(); y += sampleRate_)
  {
    for (int x = 0; x < image.size_.x(); x += sampleRate_)
    {
      const auto pixel = image.at(y, x);
      if (pixel.y_ < yThresh)
      {
        histCb[pixel.cb_]++;
        histCr[pixel.cr_]++;
      }
    }
  }

  Vector2f initialCluster;
  int maxCr = std::numeric_limits<int>::min();
  int maxCb = std::numeric_limits<int>::min();
  for (int i = 30; i < 200; i++)
  {
    if (histCb[i] > maxCb)
    {
      maxCb = histCb[i];
      initialCluster.x() = i;
    }
    if (histCr[i] > maxCr)
    {
      maxCr = histCr[i];
      initialCluster.y() = i;
    }
  }
  return initialCluster;
}

OMFCD::FieldColorCluster OMFCD::updateStep(const Image& image, const FieldColorCluster initCluster, const int maxDist, const int startY)
{
  Vector2f mean(0, 0);
  int meanY = 0;
  int count = 0;
  for (int y = startY; y < image.size_.y(); y += sampleRate_)
  {
    for (int x = 0; x < image.size_.x(); x += sampleRate_)
    {
      auto pixel = image.at(y, x);
      if (pixel.y_ < initCluster.yThresh)
      {
        const Vector2f pixelColor = Vector2f(pixel.cb_, pixel.cr_);
        const Vector2f colorErr = initCluster.mean - pixelColor;
        const int dist = (const int)(colorErr.x() * colorErr.x() + colorErr.y() * colorErr.y() * 2);
        if (dist < maxDist)
        {
          mean += pixelColor;
          meanY += pixel.y_;
          count++;
        }
      }
    }
  }
  if (count > 0)
  {
    return {mean / count, (int)(meanY / count * thresholdY_())};
  }
  return initCluster;
}

void OMFCD::sendImageForDebug(const Image& image)
{
  if (!debug().isSubscribed(mount_ + "." + imageData_->identification + "_image"))
  {
    return;
  }
  debug().update(mount_ + "." + "thresholdY", fieldColor_->thresholdY);
  debug().update(mount_ + "." + "meanCb." + imageData_->identification, fieldColor_->meanCb);
  debug().update(mount_ + "." + "meanCr." + imageData_->identification, fieldColor_->meanCr);

  if (!(counter_++ % 3))
  { // This only sends every third image because the
    // drawing takes a lot of processing time
    Image fieldColorImage(image);
    for (int y = horizonY_; y < image.size_.y(); y++)
    {
      for (int x = 0; x < image.size_.x(); x += 2)
      {
        if (fieldColor_->isFieldColor(image.at(y, x)))
        {
          fieldColorImage.at(y, x) = Color::PINK;
        }
      }
    }

    Vector2i p1(0, horizonY_);
    Vector2i p2(fieldColorImage.size_.x() - 1, horizonY_);
    fieldColorImage.line(p1, p2, Color::RED);
    debug().sendImage(mount_ + "." + imageData_->identification + "_image", fieldColorImage);
  }
}
