#include <iterator>

#include "Framework/Log/Log.hpp"
#include "Vision/ImageSegmenter/ImageSegmenter.hpp"

#include "Tools/Chronometer.hpp"
#include "Tools/Math/Statistics.hpp"
#include "Tools/Storage/Image422.hpp"

ImageSegmenter::ImageSegmenter(const ModuleManagerInterface& manager)
  : Module(manager)
  , drawEdges_(*this, "drawEdges", [] {})
  , drawFieldYellow_(*this, "drawFieldYellow", [] {})
  , drawFullImage_(*this, "drawFullImage", [] {})
  , edgeThresholdHorizontal_(*this, "edgeThresholdHorizontal", [] {})
  , edgeThresholdVertical_(*this, "edgeThresholdVertical", [] {})
  , numVerticalScanlines_(*this, "numVerticalScanlines",
                          [this] { updateVerticalScanlines_ = true; })
  , samplePointDistance_(*this, "samplePointDistance",
                         [this] { updateHorizontalScanlines_.fill(true); })
  , useMedianVerticalTop_(*this, "useMedianVerticalTop", [] {})
  , useMedianVerticalBottom_(*this, "useMedianVerticalBottom", [] {})
  , cameraMatrix_(*this)
  , fieldColor_(*this)
  , imageData_(*this)
  , robotProjection_(*this)
  , imageSegments_(*this)
{
}

void ImageSegmenter::cycle()
{
  if (!imageData_->valid)
  {
    return;
  }
  {
    Chronometer time(debug(), mount_ + "." + imageData_->identification + "_cycle_time");
    // reinitialize scanlines if the image changes
    if (updateVerticalScanlines_)
    {
      updateVerticalScanlines_ = false;
      initVerticalScanlines();
    }

    if ((imageData_->cameraPosition == CameraPosition::TOP && useMedianVerticalTop_()) ||
        (imageData_->cameraPosition == CameraPosition::BOTTOM && useMedianVerticalBottom_()))
    {
      // create the vertical scanline segments using the median of the pixel's y value and the y
      // values of the pixel above and below
      createVerticalScanlines<true>();
    }
    else
    {
      // use the pixel's y value directly
      createVerticalScanlines<false>();
    }
    if (cameraMatrix_->valid)
    {
      const int camera = static_cast<int>(imageData_->cameraPosition);
      if (updateHorizontalScanlines_[camera])
      {
        initHorizontalScanlinePositions();
        updateHorizontalScanlines_[camera] = false;
      }
      createHorizontalScanlines();
    }
    imageSegments_->valid = true;
  }
  sendDebug();
}

void ImageSegmenter::initVerticalScanlines()
{
  imageSegments_->verticalScanlines.clear();
  imageSegments_->verticalScanlines.resize(numVerticalScanlines_(),
                                           Scanline(ScanlineType::VERTICAL));
  const float scanlineSpacing =
      static_cast<float>(imageData_->image422.size.x()) / numVerticalScanlines_();

  int i = 0;
  for (auto& scanline : imageSegments_->verticalScanlines)
  {
    scanline.pos =
        static_cast<int>(scanlineSpacing * static_cast<float>(i) + scanlineSpacing / 2.f);
    scanline.id = i;
    scanline.maxIndex = imageData_->image422.size.y() - 1;
    scanline.segments.reserve(imageData_->image422.size.y());
    i++;
  }
}

template <bool useMedian>
void ImageSegmenter::createVerticalScanlines()
{
  std::vector<ScanlineState> scanlineStates;
  scanlineStates.reserve(numVerticalScanlines_());

  const int camera = static_cast<int>(imageData_->cameraPosition);
  const int edgeThreshold = edgeThresholdVertical_()[camera];

  int robotProjectionXMin = imageData_->image422.size.x();
  int robotProjectionXMax = 0;
  int lineXMin = 0;
  int lineXMax = 0;
  // If there are any RobotProjection lines visible, find lowest and highest x value
  for (const auto& line : robotProjection_->lines)
  {
    lineXMin = std::min(line.p1.x(), line.p2.x());
    lineXMax = std::max(line.p1.x(), line.p2.x());

    if (lineXMin < robotProjectionXMin)
    {
      robotProjectionXMin = lineXMin;
    }

    if (lineXMax > robotProjectionXMax)
    {
      robotProjectionXMax = lineXMax;
    }
  }

  const int horizon =
      std::min(std::min(cameraMatrix_->getHorizonHeight(0),
                        cameraMatrix_->getHorizonHeight(imageData_->image422.size.x() - 1)),
               imageData_->image422.size.y() - 1);

  for (auto& scanline : imageSegments_->verticalScanlines)
  {
    scanline.maxIndex = imageData_->image422.size.y() - 1;
    scanline.segments.emplace_back(Vector2i(scanline.pos, horizon), EdgeType::BORDER);

    // check if a robot part is visible in image, if so check it is part of the current scanline
    if (!robotProjection_->lines.empty() &&
        (robotProjectionXMin <= scanline.pos && robotProjectionXMax >= scanline.pos))
    {
      // search for the up most (smallest y) intersection of a robotProjection line with the current
      // scanline
      for (const auto& line : robotProjection_->lines)
      {
        lineXMin = std::min(line.p1.x(), line.p2.x());
        lineXMax = std::max(line.p1.x(), line.p2.x());
        if (lineXMin > scanline.pos || lineXMax < scanline.pos)
        {
          continue;
        }
        if (line.p1.x() == line.p2.x())
        {
          scanline.maxIndex = std::clamp(std::min(line.p1.y(), line.p2.y()), 0, scanline.maxIndex);
        }
        else
        {
          scanline.maxIndex = std::clamp(line.getY(scanline.pos), 0, scanline.maxIndex);
        }
      }
    }

    ScanlineState scanlineState;
    scanlineState.maxDiff = 0;
    scanlineState.peakPosition = 0;
    scanlineState.scanPoints = 0;
    scanlineState.prevYValue = imageData_->image422.at(horizon, scanline.pos).y1;
    scanlineState.prevDiff = 0;
    scanlineState.scanline = &scanline;
    scanlineStates.emplace_back(scanlineState);
  }
  const unsigned int sizeX = imageData_->image422.size.x();
  std::uint8_t yValue;
  const int upperBoundY =
      useMedian ? imageData_->image422.size.y() - 1 : imageData_->image422.size.y();
  for (int y = horizon + 2; y < upperBoundY; y += 2)
  {
    for (auto& state : scanlineStates)
    {
      if (y > state.scanline->maxIndex)
      {
        continue;
      }
      state.scanPoints++;
      if constexpr (useMedian)
      {
        const std::size_t positionInArray =
            imageData_->image422.calculateCoordPositionInArray(y - 1, state.scanline->pos);
        yValue = Statistics::median(imageData_->image422.data[positionInArray].y1,
                                    imageData_->image422.data[positionInArray + sizeX].y1,
                                    imageData_->image422.data[positionInArray + 2 * sizeX].y1);
      }
      else
      {
        yValue = imageData_->image422.at(y, state.scanline->pos).y1;
      }
      const int diff = yValue - state.prevYValue;
      detectEdge(state, y, diff, edgeThreshold);
      state.prevYValue = yValue;
      state.prevDiff = diff;
    }
  }
  // Add last segment of each scanline
  for (auto& state : scanlineStates)
  {
    const auto& scanline = *state.scanline;
    // division by 2 as the scanning above is subsampled
    const int scanPoints =
        scanline.segments.size() >= 2
            ? (scanline.maxIndex - std::next(scanline.segments.rbegin())->end.y()) / 2
            : scanline.maxIndex / 2;
    if (scanline.maxIndex > scanline.segments.front().start.y())
    {
      // it can be the case that an edge was detected before the image border was reached
      detectEdge(state, imageData_->image422.size.y(), 0, edgeThreshold);
      addSegment(Vector2i(scanline.pos, scanline.maxIndex), *state.scanline, EdgeType::BORDER,
                 scanPoints);
    }
    else
    {
      // the scanline only contains the first border segment; this will be removed
      assert(scanline.segments.size() == 1);
      state.scanline->segments.clear();
    }
  }
}

void ImageSegmenter::addSegment(const Vector2i& peakPosition, Scanline& scanline, EdgeType edgeType,
                                int scanPoints)
{
  ScanlineType& scanlineType = scanline.scanlineType;
  assert(!scanline.segments.empty());
  Segment& segment = scanline.segments.back();
  assert(peakPosition.x() >= 0 && peakPosition.y() >= 0);
  assert(scanline.scanlineType == ScanlineType::VERTICAL
             ? peakPosition.y() < imageData_->image422.size.y()
             : peakPosition.x() < imageData_->image422.size.x());
  assert(scanline.scanlineType == ScanlineType::VERTICAL ? peakPosition.y() >= segment.start.y()
                                                         : peakPosition.x() >= segment.start.x());
  segment.end = peakPosition;
  segment.endEdgeType = edgeType;
  assert(scanPoints >= 0);
  segment.scanPoints = scanPoints;
  Vector2i segmentLength = segment.end - segment.start;
  // To bit shift vectors coefficient wise
  auto shift = [](int c) { return c >> 1; };
  if ((scanlineType == ScanlineType::VERTICAL && segmentLength.y() > 5) || segmentLength.x() > 5)
  {
    // equidistant points between start and end of segment
    Vector2i spacing = segmentLength / 6;
    const YCbCr422& c1 = imageData_->image422.at(segment.start + spacing);
    const YCbCr422& c2 = imageData_->image422.at(segment.start + spacing * 2);
    const YCbCr422& c3 = imageData_->image422.at(segment.start + spacing * 3);
    const YCbCr422& c4 = imageData_->image422.at(segment.start + spacing * 4);
    const YCbCr422& c5 = imageData_->image422.at(segment.start + spacing * 5);
    segment.ycbcr422 = YCbCr422(Statistics::median(c1.y1, c2.y1, c3.y1, c4.y1, c5.y1),
                                Statistics::median(c1.cb, c2.cb, c3.cb, c4.cb, c5.cb),
                                Statistics::median(c1.y2, c2.y2, c3.y2, c4.y2, c5.y2),
                                Statistics::median(c1.cr, c2.cr, c3.cr, c4.cr, c5.cr));
  }
  else if ((scanlineType == ScanlineType::VERTICAL && segmentLength.y() > 2) ||
           segmentLength.x() > 2)
  {
    const YCbCr422& c1 = imageData_->image422.at(segment.start);
    const YCbCr422& c2 = imageData_->image422.at((segment.start + segment.end).unaryExpr(shift));
    const YCbCr422& c3 = imageData_->image422.at(segment.end);
    segment.ycbcr422 =
        YCbCr422(Statistics::median(c1.y1, c2.y1, c3.y1), Statistics::median(c1.cb, c2.cb, c3.cb),
                 Statistics::median(c1.y2, c2.y2, c3.y2), Statistics::median(c1.cr, c2.cr, c3.cr));
  }
  else
  {
    segment.ycbcr422 = imageData_->image422.at((segment.start + segment.end).unaryExpr(shift));
  }
  segment.field = fieldColor_->isFieldColor(segment.ycbcr422);
  if (edgeType != EdgeType::BORDER && edgeType != EdgeType::END)
  {
    // start a new segment if this added edge was not the last of this scanline
    scanline.segments.emplace_back(peakPosition, edgeType);
  }
}


void ImageSegmenter::initHorizontalScanlinePositions()
{
  const int camera = static_cast<int>(imageData_->cameraPosition);
  auto& scanlinePositions = imageSegments_->horizontalScanlinePositions[camera];
  scanlinePositions.clear();
  const KinematicMatrix& camera2ground = cameraMatrix_->cam2groundStand;
  const KinematicMatrix& camera2groundInv = camera2ground.inverted();
  // Distance of the sample points in meter
  const float samplePointDistance = samplePointDistance_();
  int distanceToNextScanline = 2;
  for (int y = 0; y < imageData_->image422.size.y(); y += distanceToNextScanline)
  {
    scanlinePositions.emplace_back(y);
    const Vector2i scanlinePosition(imageData_->image422.size.x() / 2, y);
    const std::optional<Vector2f> robot =
        cameraMatrix_->pixelToRobot(scanlinePosition, camera2ground);
    if (!robot.has_value())
    {
      distanceToNextScanline = 2;
      continue;
    }
    const std::optional<Vector2i> nextScanlinePosition = cameraMatrix_->robotToPixel(
        {robot->x() - samplePointDistance, robot->y()}, camera2groundInv);
    if (!nextScanlinePosition.has_value())
    {
      distanceToNextScanline = 2;
      continue;
    }
    distanceToNextScanline = std::max(nextScanlinePosition->y() - scanlinePosition.y(), 2);
  }
}

void ImageSegmenter::createHorizontalScanlines()
{
  const int horizon =
      std::min(std::min(cameraMatrix_->getHorizonHeight(0),
                        cameraMatrix_->getHorizonHeight(imageData_->image422.size.x() - 1)),
               imageData_->image422.size.y() - 1);
  if (horizon == imageData_->image422.size.y() - 1)
  {
    // horizon lies below the image --> no need to create scanlines
    return;
  }
  const int camera = static_cast<int>(imageData_->cameraPosition);
  const int edgeThreshold = edgeThresholdHorizontal_()[camera];
  int id = 0;
  for (auto& scanlinePosition : imageSegments_->horizontalScanlinePositions[camera])
  {
    if (scanlinePosition < horizon + 1)
    {
      continue;
    }
    auto& scanline = imageSegments_->horizontalScanlines.emplace_back(
        ScanlineType::HORIZONTAL, id++, scanlinePosition, imageData_->image422.size.x(),
        imageData_->image422.size.x() / 2);

    ScanlineState state;
    state.scanline = &scanline;
    state.prevYValue = imageData_->image422.at(scanline.pos, 0).y1;

    // Don't move isRobotCheckNecessary function call to the if query. Its way slower.
    const bool onRobotCheckNecessary =
        std::find_if(robotProjection_->lines.begin(), robotProjection_->lines.end(),
                     [&](const auto& line) {
                       // line crosses the y height
                       return (line.p1.y() < scanline.pos && line.p2.y() > scanline.pos) ||
                              (line.p2.y() < scanline.pos && line.p1.y() > scanline.pos);
                     }) != robotProjection_->lines.end();

    bool wasOnRobot = false;
    int lastValidX = 0;
    for (int x = 0; x < imageData_->image422.size.x(); x += 2)
    {
      if (onRobotCheckNecessary && robotProjection_->isOnRobot(Vector2i(x, scanline.pos)))
      {
        if (!wasOnRobot && !scanline.segments.empty())
        {
          // it can be the case that an edge was detected before the robot segment was reached
          detectEdge(state, x, 0, edgeThreshold);
          // if the current pixel is the first hit on the robot, end the segment.
          // as this is a segment of edge type END, no new segment will be started
          addSegment(Vector2i(x, scanline.pos), scanline, EdgeType::END, state.scanPoints);
          state.prevDiff = 0;
        }
        wasOnRobot = true;
        continue;
      }
      lastValidX = x;
      if (scanline.segments.empty())
      {
        // first pixel that is not on a robot
        scanline.segments.emplace_back(Vector2i(x, scanline.pos),
                                       x == 0 ? EdgeType::BORDER : EdgeType::START);
        state.prevYValue = imageData_->image422.at(scanline.pos, x).y1;
        state.prevDiff = 0;
        state.maxDiff = 0;
        state.peakPosition = 0;
        state.scanPoints = 0;
        wasOnRobot = false;
        continue;
      }
      if (wasOnRobot)
      {
        // The previous sample point was the last one on the robot so start a new segment.
        scanline.segments.emplace_back(Vector2i(x, scanline.pos), EdgeType::START);
        state.prevYValue = imageData_->image422.at(scanline.pos, x).y1;
        state.prevDiff = 0;
        state.maxDiff = 0;
        state.peakPosition = 0;
        state.scanPoints = 0;
        wasOnRobot = false;
        continue;
      }
      state.scanPoints++;
      const auto yValue = imageData_->image422.at(scanline.pos, x).y1;
      const int diff = yValue - state.prevYValue;
      detectEdge(state, x, diff, edgeThreshold);
      state.prevYValue = yValue;
      state.prevDiff = diff;
    }
    if (!scanline.segments.empty())
    {
      // it can be the case that an edge was detected before the image border was reached
      detectEdge(state, imageData_->image422.size.x(), 0, edgeThreshold);
      // Add the last segment
      if (wasOnRobot)
      {
        addSegment(Vector2i(lastValidX, scanline.pos), scanline, EdgeType::BORDER,
                   state.scanPoints);
      }
      else
      {
        addSegment(Vector2i(imageData_->image422.size.x() - 1, scanline.pos), scanline,
                   EdgeType::BORDER, state.scanPoints);
      }
    }
  }
}

void ImageSegmenter::detectEdge(ScanlineState& state, const int position, const int diff,
                                const int edgeThreshold)
{
  const Vector2i edgePosition = state.scanline->scanlineType == ScanlineType::VERTICAL
                                    ? Vector2i(state.scanline->pos, state.peakPosition)
                                    : Vector2i(state.peakPosition, state.scanline->pos);
  // A rising edge is detected if the difference of the two pixels exceeded the edgeThreshold for
  // one or more previous pixel pairs and falls below the edge threshold. Analog for falling edges
  if (state.prevDiff >= edgeThreshold && diff < edgeThreshold)
  {
    addSegment(edgePosition, *state.scanline, EdgeType::RISING, state.scanPoints);
    state.maxDiff = 0;
    state.scanPoints = 0;
  }
  else if (state.prevDiff <= -edgeThreshold && diff > -edgeThreshold)
  {
    addSegment(edgePosition, *state.scanline, EdgeType::FALLING, state.scanPoints);
    state.maxDiff = 0;
    state.scanPoints = 0;
  }
  if (std::abs(diff) > state.maxDiff)
  {
    state.maxDiff = std::abs(diff);
    state.peakPosition = position - 1;
  }
}

void ImageSegmenter::sendDebug()
{
  auto mount = mount_ + "." + imageData_->identification + "_vertical";
  if (debug().isSubscribed(mount))
  {
    Image vImage(Image422::get444From422Vector(imageData_->image422.size), Color::BLACK);
    if (drawFullImage_() && !imageSegments_->verticalScanlines.empty())
    {
      auto currentScanline = imageSegments_->verticalScanlines.begin();
      auto nextScanline = std::next(currentScanline);
      for (int x = 0; x < vImage.size.x(); x++)
      {
        if (nextScanline != imageSegments_->verticalScanlines.end() &&
            std::abs(x / 2 - currentScanline->pos) > std::abs(x / 2 - nextScanline->pos))
        {
          currentScanline = nextScanline;
          nextScanline = std::next(currentScanline);
        }
        for (const auto& segment : currentScanline->segments)
        {
          vImage.drawLine({x, segment.start.y()}, {x, segment.end.y()},
                          drawFieldYellow_() && segment.field > 0.f ? Color::YELLOW
                                                                    : Color(segment.ycbcr422));
        }
      }
    }
    else
    {
      for (const auto& scanline : imageSegments_->verticalScanlines)
      {
        for (const auto& segment : scanline.segments)
        {
          vImage.drawLine(Image422::get444From422Vector(segment.start),
                          Image422::get444From422Vector(segment.end),
                          drawFieldYellow_() && segment.field > 0.f ? Color::YELLOW
                                                                    : Color(segment.ycbcr422));
          if (drawEdges_())
          {
            const auto color = segment.startEdgeType == EdgeType::RISING    ? Color::RED
                               : segment.startEdgeType == EdgeType::FALLING ? Color::GREEN
                                                                            : Color::ORANGE;
            vImage.drawLine(Image422::get444From422Vector(segment.start),
                            Image422::get444From422Vector(segment.start) + Vector2i(2, 0), color);
            vImage.drawLine(Image422::get444From422Vector(segment.end),
                            Image422::get444From422Vector(segment.end) + Vector2i(2, 0), color);
          }
        }
      }
    }
    debug().sendImage(mount, vImage);
  }

  mount = mount_ + "." + imageData_->identification + "_horizontal";
  if (debug().isSubscribed(mount))
  {
    Image debugImage(Image422::get444From422Vector(imageData_->image422.size), Color::BLACK);
    if (drawFullImage_() && !imageSegments_->horizontalScanlines.empty())
    {
      auto currentScanline = imageSegments_->horizontalScanlines.begin();
      auto nextScanline = std::next(currentScanline);
      const int horizon =
          std::min(std::min(cameraMatrix_->getHorizonHeight(0),
                            cameraMatrix_->getHorizonHeight(imageData_->image422.size.x() - 1)),
                   imageData_->image422.size.y() - 1);
      for (int y = horizon; y < debugImage.size.y(); y++)
      {
        if (nextScanline != imageSegments_->horizontalScanlines.end() &&
            std::abs(y - currentScanline->pos) > std::abs(y - nextScanline->pos))
        {
          currentScanline = nextScanline;
          nextScanline = std::next(currentScanline);
        }
        for (const auto& segment : currentScanline->segments)
        {
          debugImage.drawLine({segment.start.x() * 2, y}, {segment.end.x() * 2, y},
                              drawFieldYellow_() && segment.field > 0.f ? Color::YELLOW
                                                                        : Color(segment.ycbcr422));
        }
      }
    }
    else
    {
      for (const auto& line : robotProjection_->lines)
      {
        const Line<int> line444(Image422::get444From422Vector(line.p1),
                                Image422::get444From422Vector(line.p2));
        debugImage.drawLine(line444, Color::BLUE);
      }
      for (const auto& scanline : imageSegments_->horizontalScanlines)
      {
        for (const auto& segment : scanline.segments)
        {
          debugImage.drawLine(Image422::get444From422Vector(segment.start),
                              Image422::get444From422Vector(segment.end),
                              drawFieldYellow_() && segment.field > 0.f ? Color::YELLOW
                                                                        : Color(segment.ycbcr422));
          if (drawEdges_())
          {
            const auto color = segment.startEdgeType == EdgeType::RISING    ? Color::RED
                               : segment.startEdgeType == EdgeType::FALLING ? Color::GREEN
                                                                            : Color::ORANGE;
            debugImage.drawLine(Image422::get444From422Vector(segment.start),
                                Image422::get444From422Vector(segment.start) + Vector2i(0, 2),
                                color);
            debugImage.drawLine(Image422::get444From422Vector(segment.end),
                                Image422::get444From422Vector(segment.end) + Vector2i(0, 2), color);
          }
        }
      }
    }
    debug().sendImage(mount, debugImage);
  }
}
