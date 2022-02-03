#include <cmath>

#include "Vision/PenaltySpotDetection/PenaltySpotDetection.hpp"

#include "Tools/Chronometer.hpp"

PenaltySpotDetection::PenaltySpotDetection(const ModuleManagerInterface& manager)
  : Module(manager)
  , maxPenaltySpotDetectionDistance_(*this, "maxPenaltySpotDetectionDistance", [] {})
  , minimumPenaltySpotRadius_(*this, "minimumPenaltySpotRadius", [] {})
  , requireChromaDiff_(*this, "requireChromaDiff", [] {})
  , excludeBall_(*this, "excludeBall", [] {})
  , vScanlineGapToConsider_(*this, "vScanlineGapToConsider", [] {})
  , minSpotSeedDiffY_(*this, "minSpotSeedDiffY", [] {})
  , minSpotSeedDiffChroma_(*this, "minSpotSeedDiffChroma", [] {})
  , significantYSpotSeedPointDiff_(*this, "significantYSpotSeedPointDiff", [] {})
  , significantChromaSpotSeedPointDiff_(*this, "significantChromaSpotSeedPointDiff", [] {})
  , necessarySignificantYSpotSeedPoints_(*this, "necessarySignificantYSpotSeedPoints", [] {})
  , necessarySignificantChromaSpotSeedPoints_(*this, "necessarySignificantChromaSpotSeedPoints",
                                              [] {})
  , requireFieldColor_(*this, "requireFieldColor", [] {})
  , imageData_(*this)
  , fieldDimensions_(*this)
  , cameraMatrix_(*this)
  , filteredSegments_(*this)
  , ballData_(*this)
  , fieldColor_(*this)
  , penaltySpotData_(*this)
{
}

void PenaltySpotDetection::cycle()
{
  if (!filteredSegments_->valid)
  {
    return;
  }
  {
    Chronometer time(debug(), mount_ + "." + imageData_->identification + "_cycle_time");
    detectPenaltySpot();
  }
  sendImagesForDebug();
}

void PenaltySpotDetection::detectPenaltySpot()
{
  penaltySpotSeeds_.clear();
  // Determine penalty spots max distance in image coordinates
  const std::optional<Vector2i> maxPenaltySpotDetectionImagePosition =
      cameraMatrix_->robotToPixel(Vector2f(maxPenaltySpotDetectionDistance_(), 0.f));
  if (!maxPenaltySpotDetectionImagePosition.has_value())
  {
    return;
  }
  // Search for appropriate horizontal segment
  for (const auto& hSegment : filteredSegments_->horizontal)
  {
    // Get horizontal segment length
    // 422
    assert(hSegment->end.x() >= hSegment->start.x());
    unsigned int hSegmentPixelLength = (hSegment->end.x() - hSegment->start.x());
    // Calculate the mid
    // 422
    Vector2i seed = (hSegment->end + hSegment->start).unaryExpr([](int c) { return c >> 1; });
    // Throw seed away if it is too far away
    if (seed.y() < maxPenaltySpotDetectionImagePosition->y())
    {
      continue;
    }
    assert(imageData_->image422.isInside(seed));
    // Get theoretical radius of penalty spot at that seed point
    // 444
    const auto expectedRadius = cameraMatrix_->getPixelRadius(
        imageData_->image422.size, seed, fieldDimensions_->fieldPenaltyMarkerSize / 2);
    if (!expectedRadius.has_value())
    {
      continue;
    }
    // Filter too small penalty spots (in pixel coordinates)
    if (expectedRadius.value() < minimumPenaltySpotRadius_())
    {
      continue;
    }
    // Does the segment fit?
    // 444
    assert(expectedRadius.value() > 0);
    auto ratio =
        static_cast<float>(hSegmentPixelLength) / static_cast<float>(expectedRadius.value());
    if (ratio < 0.7 || ratio > 1.3)
    {
      continue;
    }
    unsigned int vSegmentPixelLength = 0;
    // Search for appropriate vertical segment
    for (const auto& vSegment : filteredSegments_->vertical)
    {
      assert(vScanlineGapToConsider_() >= 0);
      // Skip vertical segments until reaching interesting ones
      if ((vSegment->start.x() < seed.x() - vScanlineGapToConsider_()) ||
          (vSegment->start.x() > seed.x() + vScanlineGapToConsider_()))
      {
        continue;
      }
      // Calculate difference between vertical segment and seed (mid of horizontal segment)
      int offsetFromCenter = std::abs(seed.x() - vSegment->start.x());
      assert(offsetFromCenter <= vScanlineGapToConsider_());
      // 444
      vSegmentPixelLength = vSegment->end.y() - vSegment->start.y();
      // Vertical segment can't be longer
      if (vSegmentPixelLength >= hSegmentPixelLength * 2)
      {
        continue;
      }
      const std::optional<Vector2f> rStart = cameraMatrix_->pixelToRobot(vSegment->start);
      const std::optional<Vector2f> rEnd = cameraMatrix_->pixelToRobot(vSegment->end);
      if (!rStart.has_value() || !rEnd.has_value())
      {
        continue;
      }
      assert(vSegment->start.y() <= vSegment->end.y());
      const float distance = (rEnd.value() - rStart.value()).norm();
      ratio = distance / fieldDimensions_->fieldPenaltyMarkerSize;
      // Size does matter
      if (ratio < 0.7 || ratio > 1.3)
      {
        continue;
      }
      if (excludeBall_())
      {
        // Check whether the seed is on the ball
        bool isOnBall = false;
        for (const auto& ball : ballData_->imagePositions)
        {
          if (seed.x() >= ball.center.x() - ball.radius &&
              seed.y() >= ball.center.y() - ball.radius &&
              seed.x() <= ball.center.x() + ball.radius &&
              seed.y() <= ball.center.y() + ball.radius)
          {
            isOnBall = true;
            break;
          }
        }
        // It's a ball
        if (isOnBall)
        {
          continue;
        }
      }
      const Vector2i& vMid =
          (vSegment->end + vSegment->start).unaryExpr([](int c) { return c >> 1; });
      assert(imageData_->image422.isInside(vMid));
      assert(imageData_->image422.isInside(vMid));
      // Intersection of horizontal and vertical segment should be in the middle
      if ((vMid - seed).norm() >= ((float)vSegmentPixelLength / 4))
      {
        continue;
      }
      // Intersection of horizontal and vertical segment should be in the middle
      if ((vMid - seed).norm() >= ((float)hSegmentPixelLength / 2))
      {
        continue;
      }
      // Correct the seed to the intersection
      seed.x() = vMid.x();
      // Get seed color information
      const auto& seedColor = imageData_->image422[seed];
      int seedY = static_cast<int>(seedColor.y1);
      int seedChroma = std::abs(static_cast<int>(seedColor.cb) - 128) +
                       std::abs(static_cast<int>(seedColor.cr) - 128);
      // Various directions for sample points
      const std::array<float, 12> directions = {
          {0 * TO_RAD, 30 * TO_RAD, 60 * TO_RAD, 90 * TO_RAD, 120 * TO_RAD, 150 * TO_RAD,
           180 * TO_RAD, 210 * TO_RAD, 240 * TO_RAD, 270 * TO_RAD, 300 * TO_RAD, 330 * TO_RAD}};
      const Vector2f radiusVector = Vector2f(expectedRadius.value(), vSegmentPixelLength / 2);
      bool minimumRequirementsFulfilled = true;
      bool pointOutsideTheImage = false;
      int significantYSpotSeedPoints = 0;
      int significantChromaSpotSeedPoints = 0;
      VecVector2i debugPoints;
      // Scan in various directions
      for (const auto& d : directions)
      {
        // Place thepoints outside the penalty spot
        const float sampleScale = 1.5f;
        const Vector2i& point = (seed.cast<float>() + (Vector2f(radiusVector.x() * std::cos(d) / 2,
                                                                radiusVector.y() * std::sin(d)) *
                                                       sampleScale))
                                    .cast<int>();
        // Point still in image?
        if (!imageData_->image422.isInside(point))
        {
          pointOutsideTheImage = true;
          break;
        }
        debugPoints.push_back(point);
        // Get color information of the point
        const auto& pointColor = imageData_->image422[point];
        const int pointY = static_cast<int>(std::max(pointColor.y1, pointColor.y2));
        const int pointChroma = std::abs(static_cast<int>(pointColor.cb) - 128) +
                                std::abs(static_cast<int>(pointColor.cr) - 128);
        // Calculae some diffs between seed and point
        const int diffY = seedY - pointY;
        const int diffChroma = pointChroma - seedChroma;
        // Sufficient luminance diff? It must be darker outside the penalty spot
        if (diffY < minSpotSeedDiffY_())
        {
          minimumRequirementsFulfilled = false;
          break;
        }
        // Sufficient chroma diff? It muss be more colorful outside the penalty spot
        if (requireChromaDiff_() && diffChroma < minSpotSeedDiffChroma_())
        {
          minimumRequirementsFulfilled = false;
          break;
        }
        if (requireFieldColor_() && !fieldColor_->isFieldColor(pointColor))
        {
          minimumRequirementsFulfilled = false;
          break;
        }
        // Count intense diffs in luminance
        if (diffY > significantYSpotSeedPointDiff_())
        {
          significantYSpotSeedPoints++;
        }
        // Count intense diffs in chroma
        if (diffChroma > significantChromaSpotSeedPointDiff_())
        {
          significantChromaSpotSeedPoints++;
        }
      }
      // Require all sample points in the image
      if (pointOutsideTheImage)
      {
        continue;
      }
      // Are the minimum diffs fulfilled? It may be brighter or less colorful outside
      if (!minimumRequirementsFulfilled)
      {
        continue;
      }
      // Check whether enough intense luminance diffs are present
      if (significantYSpotSeedPoints < necessarySignificantYSpotSeedPoints_())
      {
        continue;
      }
      // Check whether enough intense chroma diffs are present
      if (significantChromaSpotSeedPoints < necessarySignificantChromaSpotSeedPoints_())
      {
        continue;
      }
      // Create penalty spot
      penaltySpotSeeds_.emplace_back(seed);
      penaltySpotSeeds_.back().hSegment = hSegment;
      penaltySpotSeeds_.back().vSegment = vSegment;
      penaltySpotSeeds_.back().score =
          significantYSpotSeedPoints + significantChromaSpotSeedPoints - offsetFromCenter;
      penaltySpotSeeds_.back().width = hSegmentPixelLength;
      penaltySpotSeeds_.back().height = vSegmentPixelLength;
      penaltySpotSeeds_.back().expectedRadius = expectedRadius.value();
      penaltySpotSeeds_.back().debugPoints = debugPoints;
    }
  }
  if (penaltySpotSeeds_.empty())
  {
    return;
  }
  // Sort penalty points by score
  std::sort(penaltySpotSeeds_.begin(), penaltySpotSeeds_.end(),
            [](const PenaltySpot& penaltySpot1, const PenaltySpot& penaltySpot2) {
              return (penaltySpot1.score > penaltySpot2.score);
            });
  assert(penaltySpotSeeds_.front().score >= penaltySpotSeeds_.back().score);
  // Take penalty point with highest score
  const std::optional<Vector2f> robotCoordinates =
      cameraMatrix_->pixelToRobot(penaltySpotSeeds_[0].pixelPosition);
  if (robotCoordinates.has_value())
  {
    penaltySpotSeeds_[0].relativePosition = robotCoordinates.value();
    penaltySpotData_->penaltySpot = penaltySpotSeeds_[0];
    penaltySpotData_->valid = true;
    penaltySpotData_->timestamp = imageData_->captureTimePoint;
  }
}

void PenaltySpotDetection::sendImagesForDebug()
{
  auto mount = mount_ + "." + imageData_->identification + "_image_penaltySpot";
  if (debug().isSubscribed(mount))
  {
    Image image(imageData_->image422.to444Image());
    for (const auto& spotSeed : penaltySpotSeeds_)
    {
      image.drawEllipse(
          Image422::get444From422Vector(spotSeed.pixelPosition),
          Image422::get444From422Vector(Vector2i(spotSeed.width / 2, spotSeed.height / 2)), 0,
          Color::ORANGE, 100);
      image.drawCircle(Image422::get444From422Vector(spotSeed.pixelPosition), 2, Color::ORANGE);
      image.drawLine(Image422::get444From422Vector(spotSeed.hSegment->start),
                     Image422::get444From422Vector(spotSeed.hSegment->end), Color::ORANGE);
      image.drawLine(Image422::get444From422Vector(spotSeed.vSegment->start),
                     Image422::get444From422Vector(spotSeed.vSegment->end), Color::ORANGE);
    }
    if (penaltySpotData_->valid)
    {
      image.drawEllipse(
          Image422::get444From422Vector(penaltySpotData_->penaltySpot.pixelPosition),
          Image422::get444From422Vector(Vector2i(penaltySpotData_->penaltySpot.width / 2,
                                                 penaltySpotData_->penaltySpot.height / 2)),
          0, Color::RED, 100);
      image.drawCircle(Image422::get444From422Vector(penaltySpotData_->penaltySpot.pixelPosition),
                       2, Color::RED);
      image.drawLine(Image422::get444From422Vector(penaltySpotData_->penaltySpot.hSegment->start),
                     Image422::get444From422Vector(penaltySpotData_->penaltySpot.hSegment->end),
                     Color::RED);
      image.drawLine(Image422::get444From422Vector(penaltySpotData_->penaltySpot.vSegment->start),
                     Image422::get444From422Vector(penaltySpotData_->penaltySpot.vSegment->end),
                     Color::RED);
      image.drawText(std::to_string((int)penaltySpotData_->penaltySpot.score),
                     Image422::get444From422Vector(penaltySpotData_->penaltySpot.pixelPosition),
                     Color::BLACK);
      for (const auto& point : penaltySpotData_->penaltySpot.debugPoints)
      {
        image.drawCircle(Image422::get444From422Vector(point), 3, Color::RED);
      }
    }
    debug().sendImage(mount, image);
  }

  mount = mount_ + "." + imageData_->identification + "_image_chroma";
  if (debug().isSubscribed(mount))
  {
    Image chromaImage(Image422::get444From422Vector(imageData_->image422.size));
    for (int y = 0; y < chromaImage.size.y(); ++y)
    {
      for (int x = 0; x < chromaImage.size.x(); ++x)
      {
        int cb = imageData_->image422.at(y, x / 2).cb;
        int cr = imageData_->image422.at(y, x / 2).cr;
        int sat = std::abs(cb - 128) + std::abs(cr - 128);
        chromaImage.at(y, x).y = sat;
        chromaImage.at(y, x).cb = 128;
        chromaImage.at(y, x).cr = 128;
      }
    }
    debug().sendImage(mount, chromaImage);
  }
}
