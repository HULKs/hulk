#include "BoxCandidatesProvider.hpp"
#include "Tools/Chronometer.hpp"
#include "print.h"
#include <algorithm>


BoxCandidatesProvider::BoxCandidatesProvider(const ModuleManagerInterface& manager)
  : Module(manager)
  , cameraMatrix_(*this)
  , imageData_(*this)
  , integralImageData_(*this)
  , fieldBorder_(*this)
  , fieldColor_(*this)
  , fieldDimensions_(*this)
  , robotProjection_(*this)

  , blockSize_(
        *this, "blockSize", [] {}, [this] { return imageData_->camera == Camera::TOP; })
  , brightPixelThreshold_(
        *this, "brightPixelThreshold", [] {}, [this] { return imageData_->camera == Camera::TOP; })
  , darkPixelThreshold_(
        *this, "darkPixelThreshold", [] {}, [this] { return imageData_->camera == Camera::TOP; })
  , innerRadiusScale_(
        *this, "innerRadiusScale", [] {}, [this] { return imageData_->camera == Camera::TOP; })
  , outerRadiusScale_(
        *this, "outerRadiusScale", [&] { assert(outerRadiusScale_() > 1.f); },
        [this] { return imageData_->camera == Camera::TOP; })
  , maxCandidateNumber_(
        *this, "maxCandidateNumber", [] {}, [this] { return imageData_->camera == Camera::TOP; })
  , mergeToleranceFactor_(
        *this, "mergeToleranceFactor", [] {}, [this] { return imageData_->camera == Camera::TOP; })
  , minBoxRating_(
        *this, "minBoxRating", [] {}, [this] { return imageData_->camera == Camera::TOP; })
  , minPixelRadius_(
        *this, "minPixelRadius", [] {}, [this] { return imageData_->camera == Camera::TOP; })
  , numberBrightPixels_(
        *this, "numberBrightPixels", [] {}, [this] { return imageData_->camera == Camera::TOP; })
  , numberDarkPixels_(
        *this, "numberDarkPixels", [] {}, [this] { return imageData_->camera == Camera::TOP; })
  , maxNumberFieldPixels_(
        *this, "maxNumberFieldPixels", [] {}, [this] { return imageData_->camera == Camera::TOP; })
  , sampleSize_(*this, "sampleSize", [] {})
  , skipOutsideField_(
        *this, "skipOutsideField", [] {}, [this] { return imageData_->camera == Camera::TOP; })
  , stepsPerBallSize_(
        *this, "stepsPerBallSize", [] {}, [this] { return imageData_->camera == Camera::TOP; })

  , boxCandidates_(*this)
{
  assert(outerRadiusScale_() > 1.f);
}

void BoxCandidatesProvider::cycle()
{
  {
    Chronometer time(debug(), mount_ + ".cycle_time." + imageData_->identification);
    if (!integralImageData_->valid || !cameraMatrix_->valid)
    {
      return;
    }

    std::vector<CandidateBox> candidateBoxes;
    findCandidateBoxes(candidateBoxes);
    std::vector<Circle<int>> candidateCircles = getBestCandidates(candidateBoxes);

    for (auto& circle : candidateCircles)
    {
      std::vector<std::uint8_t> sample;
      bool ok = sampleBoundingBox(circle, sampleSize_(), sample);
      if (ok)
      {
        boxCandidates_->candidates.emplace_back(circle, sample);
      }
    }
  }
  sendDebug();
}

void BoxCandidatesProvider::calculateBlockRating(int& blockY, int& blockX,
                                                 CandidateBox& bestResult) const
{
  // integral image coordintates of the current block
  const Vector2i integralTopLeft(blockX * blockSize_() / integralImageData_->image.scale,
                                 blockY * blockSize_() / integralImageData_->image.scale);
  const Vector2i integralBottomRight(
      (blockX * blockSize_() + blockSize_()) / integralImageData_->image.scale,
      (blockY * blockSize_() + blockSize_()) / integralImageData_->image.scale);
  const Rectangle<int> integralBlock(integralTopLeft, integralBottomRight);

  // position of the center in the original 422 image
  const Vector2i pixelCenterPosition =
      Image422::get422From444Vector(integralBlock.center() * integralImageData_->image.scale);
  // skip this block if it's outside the field
  if (!fieldBorder_->isInsideField(pixelCenterPosition) && skipOutsideField_())
  {
    return;
  }
  if (robotProjection_->isOnRobot(pixelCenterPosition))
  {
    return;
  }
  // estimated ball radius in pixel valid for the entire block to reduce computation
  int pixelRadius = 0;
  if (!cameraMatrix_->getPixelRadius(imageData_->image422.size, pixelCenterPosition,
                                     fieldDimensions_->ballDiameter / 2, pixelRadius))
  {
    Log(LogLevel::ERROR) << "Projection failed!";
    return;
  }
  // save the estimated radius in bestResult
  bestResult.boxRadius = pixelRadius;

  // skip this block if the estimated ball radius is smaller than minPixelRadius_
  if (pixelRadius < minPixelRadius_())
  {
    return;
  }
  // calculate a dynamic step size to reduce computation
  const int stepSize = std::max(pixelRadius / stepsPerBallSize_(), 1);
  const int innerRadius = static_cast<int>(
      std::ceil(static_cast<float>(pixelRadius) /
                static_cast<float>(integralImageData_->image.scale) * innerRadiusScale_()));
  const int outerRadius = static_cast<int>(static_cast<float>(innerRadius) * outerRadiusScale_());

  for (int integralY = integralBlock.topLeft.y(); integralY < integralBlock.bottomRight.y();
       integralY += stepSize)
  {
    for (int integralX = integralBlock.topLeft.x(); integralX < integralBlock.bottomRight.x();
         integralX += stepSize)
    {
      const int rating = getRating(integralX, integralY, innerRadius, outerRadius);
      if (rating > bestResult.rating)
      {
        bestResult.rating = rating;
        bestResult.pos.x() = integralX * integralImageData_->image.scale;
        bestResult.pos.y() = integralY * integralImageData_->image.scale;
      }
    }
  }
}

void BoxCandidatesProvider::findCandidateBoxes(std::vector<CandidateBox>& candidates)
{
  // the projected horizon y position in the current image
  const int horizon =
      std::min(std::min(cameraMatrix_->getHorizonHeight(0),
                        cameraMatrix_->getHorizonHeight(imageData_->image422.size.x() - 1)),
               imageData_->image422.size.y() - 1);
  // size of the original image in pixel
  const Vector2i pixelImageSize = Image422::get444From422Vector(imageData_->image422.size);

  for (int blockY = static_cast<int>(std::ceil(horizon / static_cast<float>(blockSize_())));
       blockY < pixelImageSize.y() / blockSize_(); blockY++)
  {
    for (int blockX = 0; blockX < pixelImageSize.x() / blockSize_(); blockX++)
    {
      CandidateBox box;
      calculateBlockRating(blockY, blockX, box);
      if (box.rating > minBoxRating_())
      {
        candidates.push_back(box);
        boxCandidates_->debugBoxes.emplace_back(Circle<int>(box.pos, box.boxRadius), Color::BLUE);
      }
    }
  }
}

int BoxCandidatesProvider::getRating(int& integralX, int& integralY, const int& innerRadius,
                                     const int& outerRadius) const
{
  // multiply all values by 2**shift to get a larger range
  const unsigned int shift = 7;
  // boundaries of the inner and outer boxes clamped between 0 and the respective maximum image size
  const Vector2i p1(std::max(integralX - innerRadius, 0), std::max(integralY - innerRadius, 0));
  const Vector2i p2(std::min(integralX + innerRadius, integralImageData_->image.size.x() - 1),
                    std::min(integralY + innerRadius, integralImageData_->image.size.y() - 1));
  const unsigned int innerValue = integralImageData_->getIntegralValue(p1, p2) << shift;
  const unsigned int innerArea = std::max((p2.x() - p1.x()) * (p2.y() - p1.y()), 1);

  const Vector2i p3(std::max(integralX - outerRadius, 0), std::max(integralY - outerRadius, 0));
  const Vector2i p4(std::min(integralX + outerRadius, integralImageData_->image.size.x() - 1),
                    std::min(integralY + outerRadius, integralImageData_->image.size.y() - 1));
  const unsigned int outerValue = integralImageData_->getIntegralValue(p3, p4) << shift;
  const unsigned int outerArea = std::max((p4.x() - p3.x()) * (p4.y() - p3.y()), 1);

  const int rating =
      static_cast<int>(innerValue / innerArea) - static_cast<int>(outerValue / outerArea);
  return rating;
}

std::vector<Circle<int>>
BoxCandidatesProvider::getBestCandidates(std::vector<CandidateBox>& candidates) const
{
  std::vector<Circle<int>> candidateCircles;
  std::sort(candidates.begin(), candidates.end(),
            [](const CandidateBox& a, const CandidateBox& b) { return a.rating > b.rating; });
  int candidatesCount = 0;
  for (const auto& box : candidates)
  {
    if (candidatesCount == maxCandidateNumber_())
    {
      break;
    }
    if (!isInsideCandidate(box.pos, candidateCircles))
    {
      candidateCircles.emplace_back(Image422::get422From444Vector(box.pos), box.boxRadius);
      candidatesCount++;
    }
  }
  return candidateCircles;
}

bool BoxCandidatesProvider::isInsideCandidate(const Vector2i& pos,
                                              const std::vector<Circle<int>>& circles) const
{
  for (const auto& circle : circles)
  {
    if ((Image422::get444From422Vector(circle.center) - pos).norm() <
        circle.radius * (1.f + mergeToleranceFactor_()))
    {
      return true;
    }
  }
  return false;
}

bool BoxCandidatesProvider::sampleBoundingBox(const Circle<int>& circle,
                                              const unsigned int sampleSize,
                                              std::vector<std::uint8_t>& colorSampled)
{
  const Vector2i from(circle.center.x() * 2 - circle.radius, circle.center.y() - circle.radius);
  const float scale = circle.radius * 2.0f / sampleSize;
  int numberDarkPixels = 0;
  int numberBrightPixels = 0;
  int numFieldColor = 0;

  colorSampled.resize(sampleSize * sampleSize * 3);

  Vector2i pixel(from);
  for (unsigned int y = 0; y < sampleSize; y++)
  {
    pixel.y() = from.y() + static_cast<int>(y * scale);
    for (unsigned int x = 0; x < sampleSize; x++)
    {
      // First, calculate x position in YUV444 coords
      pixel.x() = from.x() + static_cast<int>(x * scale);
      // Check if 444 coord is even
      const bool xEven = pixel.x() % 2 == 0;
      // Convert to 422 coordinate
      pixel.x() /= 2;
      // Calculate coordinate in sampled array
      const unsigned int pos = y * sampleSize * 3 + x * 3;
      // Fallback to 128 if pixel is not inside image
      if (!imageData_->image422.isInside(pixel))
      {
        const auto fallback = static_cast<std::uint8_t>(128);
        colorSampled[pos] = fallback;
        colorSampled[pos + 1] = fallback;
        colorSampled[pos + 2] = fallback;
        continue;
      }
      // Get 422 Color
      const YCbCr422& color = imageData_->image422[pixel];
      // If 444 coord was even, take the first y value. Otherwise the second
      const std::uint8_t& yByte = xEven ? color.y1_ : color.y2_;
      colorSampled[pos] = yByte;
      colorSampled[pos + 1] = color.cb_;
      colorSampled[pos + 2] = color.cr_;
      if (yByte > brightPixelThreshold_())
      {
        numberBrightPixels++;
      }
      if (yByte < darkPixelThreshold_() && fieldColor_->isFieldColor(color) == 0.f)
      {
        numberDarkPixels++;
      }
    }
  }
  for (unsigned int y = 0; y < sampleSize; y++)
  {
    for (unsigned int x = 3; x < sampleSize - 3; x++)
    {
      const unsigned int pos = y * sampleSize * 3 + x * 3;
      const bool isFieldColor =
          fieldColor_->isFieldColor(YCbCr422(colorSampled[pos], colorSampled[pos + 1],
                                             colorSampled[pos], colorSampled[pos + 2])) > 0.f;
      if (isFieldColor)
      {
        numFieldColor++;
      }
    }
  }
  const bool exceedingFieldPixels = numFieldColor > maxNumberFieldPixels_();
  if (exceedingFieldPixels)
  {
    Circle<int> tmp(Image422::get444From422Vector(circle.center), circle.radius);
    boxCandidates_->debugBoxes.emplace_back(tmp, Color::RED);
  }
  const bool enoughDarkPixels = numberDarkPixels >= numberDarkPixels_();
  const bool enoughBrightPixels = numberBrightPixels >= numberBrightPixels_();
  return enoughDarkPixels && enoughBrightPixels && !exceedingFieldPixels;
}

void BoxCandidatesProvider::sendDebug() const
{
  const std::string debugImageMount = mount_ + "." + imageData_->identification + "_blockSize";
  if (debug().isSubscribed(debugImageMount))
  {
    Image debugImage(imageData_->image422.to444Image());
    // the projected horizon y position in the current image
    const int horizon =
        std::min(std::min(cameraMatrix_->getHorizonHeight(0),
                          cameraMatrix_->getHorizonHeight(imageData_->image422.size.x() - 1)),
                 imageData_->image422.size.y() - 1);
    // size of the original image in pixel
    const Vector2i pixelImageSize = Image422::get444From422Vector(imageData_->image422.size);

    for (int blockY = static_cast<int>(std::ceil(horizon / static_cast<float>(blockSize_())));
         blockY < pixelImageSize.y() / blockSize_(); blockY++)
    {
      debugImage.line(Vector2i(0, blockY * blockSize_()),
                      Vector2i(pixelImageSize.x(), blockY * blockSize_()), Color::RED);
    }
    for (int blockX = 0; blockX < pixelImageSize.x() / blockSize_(); blockX++)
    {
      debugImage.line(Vector2i(blockX * blockSize_(), 0),
                      Vector2i(blockX * blockSize_(), pixelImageSize.y()), Color::RED);
    }
    debug().sendImage(debugImageMount, debugImage);
  }
}
