#include "Vision/PerspectiveGridCandidates/PerspectiveGridCandidatesProvider.hpp"
#include "Framework/Log/Log.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Math/Eigen.hpp"
#include <algorithm>
#include <iterator>
#include <set>


PerspectiveGridCandidatesProvider::PerspectiveGridCandidatesProvider(
    const ModuleManagerInterface& manager)
  : Module(manager)
  , imageData_(*this)
  , cameraMatrix_(*this)
  , fieldDimensions_(*this)
  , filteredSegments_(*this)
  , lineData_(*this)
  , minimumRadius_(*this, "minimumRadius", [] {})
  , maximumCandidates_(*this, "maximumCandidates", [] {})
  , perspectiveGridCandidates_(*this)
  , horizonY_{0}
  , numberOfCircles_{0}
{
}

void PerspectiveGridCandidatesProvider::cycle()
{
  {
    Chronometer time(debug(), mount_ + ".cycle_time." + imageData_->identification);
    if (!imageData_->valid || !cameraMatrix_->valid)
    {
      return;
    }

    // first generate rows of circles
    generateCircleRows();
    // second match filtered segment centers to circles in generated circle rows and generate
    // candidate circles
    generateCandidates();
  }

  sendDebugImage();
}

void PerspectiveGridCandidatesProvider::generateCircleRows()
{
  // the projected horizon y position in the current image
  const auto horizonLeft =
      std::clamp(cameraMatrix_->getHorizonHeight(0), 0, imageData_->image422.size.y() - 1);
  const auto horizonRight =
      std::clamp(cameraMatrix_->getHorizonHeight(imageData_->image422.size.x() - 1), 0,
                 imageData_->image422.size.y() - 1);
  const auto horizonX = horizonLeft < horizonRight ? 0 : imageData_->image422.size.x() - 1;
  horizonY_ = horizonLeft < horizonRight ? horizonLeft : horizonRight;

  circleRows_.clear();
  numberOfCircles_ = 0;
  int radius444 = 42; // initial radius
                      // (only used if all projections will fail, otherwise overwritten)
  for (int centerLineY = imageData_->image422.size.y() - 1; centerLineY >= horizonY_;
       centerLineY -= radius444 * 2)
  {
    // continue with unchanged/non-decreased radius if projection fails
    radius444 = cameraMatrix_
                    ->getPixelRadius(imageData_->image422.size, {horizonX, centerLineY},
                                     fieldDimensions_->ballDiameter / 2)
                    .value_or(radius444);
    if (radius444 < minimumRadius_())
    {
      break;
    }
    circleRows_.emplace_back(centerLineY, radius444);
    numberOfCircles_ +=
        imageData_->image422.size.x() / radius444; // radius is 444, therefore a 422 diameter
  }
}

void PerspectiveGridCandidatesProvider::generateCandidates()
{
  if (circleRows_.empty())
  {
    perspectiveGridCandidates_->valid = true;
    return;
  }

  for (auto segmentIterator = filteredSegments_->vertical.begin();
       segmentIterator != filteredSegments_->vertical.end(); ++segmentIterator)
  {
    const auto filteredSegmentCenter = ((*segmentIterator)->start + (*segmentIterator)->end) / 2;

    // only consider vertical filtered segments unused in line detection
    if (lineData_->usedVerticalFilteredSegments[std::distance(filteredSegments_->vertical.begin(),
                                                              segmentIterator)])
    {
      continue;
    }

    // find radius by finding lower bound while iterating backwards (in positive y-direction)
    auto circleRowOfFilteredSegmentCenter = std::lower_bound(
        circleRows_.rbegin(), circleRows_.rend(), filteredSegmentCenter.y(),
        [](const CircleRow& element, int value) { return element.centerLineY < value; });
    // at this point circleRows is non-empty
    // if we got rend()-iterator from lower_bound (no lower bound found), we use the last center
    // line
    if (circleRowOfFilteredSegmentCenter == circleRows_.rend())
    {
      --circleRowOfFilteredSegmentCenter;
    }
    // check if bound is correctly matched (i.e. value is near bound)
    // the threshold is bound - radius (moving the bound up by radius)
    if (filteredSegmentCenter.y() <
        circleRowOfFilteredSegmentCenter->centerLineY - circleRowOfFilteredSegmentCenter->radius444)
    {
      // value is more near previous bound => try correct the bound
      // if we are already at the uppest position (lowest stored y-coordinate)
      if (circleRowOfFilteredSegmentCenter == circleRows_.rbegin())
      {
        // bound cannot be corrected because it is already the minimum of stored radii
        // skip this segment because there is no circle containing it
        continue;
      }
      // there are stored radii before the bound => move bound up by 1
      --circleRowOfFilteredSegmentCenter;
    }
    // diameter in 422 coordinates (radius is in y-direction, i.e. 444 coordinates)
    const auto diameter422 = static_cast<float>(circleRowOfFilteredSegmentCenter->radius444);
    // calculate x-coordinate (using integer division)
    const auto x =
        std::floor(static_cast<float>(filteredSegmentCenter.x() + diameter422 / 2) / diameter422) *
        diameter422;
    // add candidate (since candidates is an std::unordered_set, adding it multiple times does not
    // break anything)
    perspectiveGridCandidates_->candidates.emplace(
        Vector2i{static_cast<int>(x), circleRowOfFilteredSegmentCenter->centerLineY},
        circleRowOfFilteredSegmentCenter->radius444);
  }

  // limit number of generated candidates
  if (perspectiveGridCandidates_->candidates.size() > maximumCandidates_())
  {
    auto newBegin = perspectiveGridCandidates_->candidates.begin();
    std::advance(newBegin, perspectiveGridCandidates_->candidates.size() - maximumCandidates_());
    perspectiveGridCandidates_->candidates.erase(perspectiveGridCandidates_->candidates.begin(),
                                                 newBegin);
  }

  perspectiveGridCandidates_->valid = true;
}

void PerspectiveGridCandidatesProvider::sendDebugImage() const
{
  const std::string debugImageMount = mount_ + "." + imageData_->identification;

  if (debug().isSubscribed(debugImageMount))
  {
    Image debugImage(imageData_->image422.to444Image());
    debugImage.drawLine({0, horizonY_}, {debugImage.size.x() - 1, horizonY_}, Color::RED);
    debugImage.drawLine({0, cameraMatrix_->getHorizonHeight(0)},
                        {debugImage.size.x() - 1,
                         cameraMatrix_->getHorizonHeight(imageData_->image422.size.x() - 1)},
                        Color::PINK);

    for (const auto& circleRow : circleRows_)
    {
      for (int x = 0; x < debugImage.size.x() - 1 + 2 * circleRow.radius444;
           x += 2 * circleRow.radius444)
      {
        debugImage.drawCircle({x, circleRow.centerLineY}, circleRow.radius444, Color::BLUE);
      }
    }

    for (auto segmentIterator = filteredSegments_->vertical.begin();
         segmentIterator != filteredSegments_->vertical.end(); ++segmentIterator)
    {
      const auto filteredSegmentCenter = ((*segmentIterator)->start + (*segmentIterator)->end) / 2;

      if (lineData_->usedVerticalFilteredSegments[std::distance(filteredSegments_->vertical.begin(),
                                                                segmentIterator)])
      {
        debugImage.drawCross(Image422::get444From422Vector(filteredSegmentCenter), 3, Color::RED);
      }
      else
      {
        debugImage.drawCross(Image422::get444From422Vector(filteredSegmentCenter), 3, Color::GREEN);
      }
    }

    for (const auto& candidate : perspectiveGridCandidates_->candidates)
    {
      const auto circle = candidate.get444from422();
      debugImage.drawRectangle(circle.center, circle.radius * 2, circle.radius * 2, Color::GREEN);
    }

    debugImage.drawText(
        "#filtered segments: " + std::to_string(filteredSegments_->vertical.size()) +
            "\n#circle rows: " + std::to_string(circleRows_.size()) +
            "\n#circles: " + std::to_string(numberOfCircles_) +
            "\n#candidates: " + std::to_string(perspectiveGridCandidates_->candidates.size()),
        Vector2i::Zero(), Color::YELLOW);

    debug().sendImage(debugImageMount, debugImage);
  }
}
