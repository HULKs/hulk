#include "Tools/Chronometer.hpp"
#include "Tools/Storage/Image.hpp"

#include "OneMeansFieldColorDetection.hpp"

#include <limits>

using OMFCD = OneMeansFieldColorDetection;

OMFCD::OneMeansFieldColorDetection(const ModuleManagerInterface& manager)
  : Module(manager)
  , calculateInitialGuess_(*this, "calculateInitialGuess",
                           [this] {
                             if (this->calculateInitialGuess_())
                             {
                               updateInitialGuessBottom_ = true;
                               updateInitialGuessTop_ = true;
                               this->calculateInitialGuess_() = false;
                             }
                           })
  , initialGuessTop_(*this, "initialGuessTop", [] {})
  , initialGuessBottom_(*this, "initialGuessBottom", [] {})
  , thresholdYParam_(*this, "thresholdY", [] {})
  , thresholdUV_(*this, "thresholdUV", [] {})
  , sampleRate_(10)
  , imageData_(*this)
  , cameraMatrix_(*this)
  , fieldColor_(*this)
  , updateInitialGuessTop_(false)
  , updateInitialGuessBottom_(false)
{
  fieldColor_->isFieldColor = [this](const YCbCr422& pixel) -> float {
    const int cb = (pixel.cb_ - meanCb_);
    const int cr = (pixel.cr_ - meanCr_);
    return (pixel.y1_ < thresholdY_ && pixel.y2_ < thresholdY_ &&
            cb * cb + cr * cr * 2 < thresholdUvSquared_) *
           1.f;
  };
}

void OMFCD::cycle()
{
  Chronometer time(debug(), mount_ + ".cycleTime");
  const Image422& image = imageData_->image422;

  horizonY_ = cameraMatrix_->getHorizonHeight();
  if (horizonY_ >= image.size.y())
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
      value << initialStep(imageData_->image422, 200, horizonY_);
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
      value << initialStep(imageData_->image422, 200, horizonY_);
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
  thresholdUvSquared_ = thresholdUVSquared;
  thresholdY_ = color.yThresh;
  meanCb_ = static_cast<int>(color.mean.x());
  meanCr_ = static_cast<int>(color.mean.y());
  fieldColor_->valid = true;

  sendImageForDebug(image);
}

Vector2f OMFCD::initialStep(const Image422& image, const int yThresh, const int startY) const
{
  std::array<int, 256> histCb = {{}};
  std::array<int, 256> histCr = {{}};
  for (int y = startY; y < image.size.y(); y += sampleRate_)
  {
    for (int x = 0; x < image.size.x(); x += sampleRate_ / 2)
    {
      const auto pixel = image.at(y, x);
      if (pixel.y1_ < yThresh)
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

OMFCD::FieldColorCluster OMFCD::updateStep(const Image422& image,
                                           const FieldColorCluster initCluster, const int maxDist,
                                           const int startY)
{
  Vector2f mean(0, 0);
  int meanY = 0;
  int count = 0;
  for (int y = startY; y < image.size.y(); y += sampleRate_)
  {
    for (int x = 0; x < image.size.x(); x += sampleRate_ / 2)
    {
      auto pixel = image.at(y, x);
      if (pixel.y1_ < initCluster.yThresh)
      {
        const Vector2f pixelColor = Vector2f(pixel.cb_, pixel.cr_);
        const Vector2f colorErr = initCluster.mean - pixelColor;
        const int dist = static_cast<int>(colorErr.x() * colorErr.x() + colorErr.y() * colorErr.y() * 2);
        if (dist < maxDist)
        {
          mean += pixelColor;
          meanY += pixel.y1_;
          count++;
        }
      }
    }
  }
  if (count > 0)
  {
    return {mean / count, (int)(meanY / count * thresholdYParam_())};
  }
  return initCluster;
}

void OMFCD::sendImageForDebug(const Image422& image)
{
  if (!debug().isSubscribed(mount_ + "." + imageData_->identification + "_image"))
  {
    return;
  }
  debug().update(mount_ + "." + "thresholdY", thresholdY_);
  debug().update(mount_ + "." + "meanCb." + imageData_->identification, meanCb_);
  debug().update(mount_ + "." + "meanCr." + imageData_->identification, meanCr_);

  if (!(counter_++ % 3))
  { // This only sends every third image because the
    // drawing takes a lot of processing time
    Image fieldColorImage(image.to444Image());
    for (int y = horizonY_; y < fieldColorImage.size_.y(); y += 2)
    {
      for (int x = 0; x < fieldColorImage.size_.x(); ++x)
      {
        if (fieldColor_->isFieldColor(image.at(y, x / 2)))
        {
          fieldColorImage.at(Vector2i(x, y)) = Color::PINK;
        }
      }
    }

    Vector2i p1(0, horizonY_);
    Vector2i p2(fieldColorImage.size_.x() - 1, horizonY_);
    fieldColorImage.line(p1, p2, Color::RED);
    debug().sendImage(mount_ + "." + imageData_->identification + "_image", fieldColorImage);
  }
}
