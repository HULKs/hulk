#include <iterator>

#include "ImageSegmenter.hpp"
#include "print.hpp"

#include "Tools/Chronometer.hpp"
#include "Tools/Math/ColorConverter.hpp"
#include "Tools/Math/Statistics.hpp"
#include "Tools/Storage/Image422.hpp"

ImageSegmenter::ImageSegmenter(const ModuleManagerInterface& manager)
  : Module(manager)
  , updateScanlines_(false)
  , scanGridsValid_({{false, false}})
  , drawFullImage_(*this, "drawFullImage", [] {})
  , edgeThresholdHorizontal_(*this, "edgeThresholdHorizontal", [] {})
  , edgeThresholdVertical_(*this, "edgeThresholdVertical", [] {})
  , numScanlines_(*this, "numScanlines", [this] { updateScanlines_ = true; })
  , drawEdges_(*this, "drawEdges", [] {})
  , useMedianVerticalTop_(*this, "useMedianVerticalTop", [] {})
  , useMedianVerticalBottom_(*this, "useMedianVerticalBottom", [] {})
  , imageData_(*this)
  , cameraMatrix_(*this)
  , fieldColor_(*this)
  , robotProjection_(*this)
  , imageSegments_(*this)
{
}

void ImageSegmenter::cycle()
{
  {
    Chronometer time(debug(), mount_ + "." + imageData_->identification + "_cycle_time");
    if ((imageData_->camera == Camera::TOP && useMedianVerticalTop_()) ||
        (imageData_->camera == Camera::BOTTOM && useMedianVerticalBottom_()))
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
    createHorizontalScanlines();
    imageSegments_->valid = true;
  }
  sendDebug();
}

void ImageSegmenter::calculateScanGrids()
{
  if (!imageData_->is_provided || !cameraMatrix_->valid)
  {
    return;
  }
  const int camera = static_cast<int>(imageData_->camera);
  const KinematicMatrix& camera2ground = cameraMatrix_->cam2groundStand;
  const KinematicMatrix& camera2ground_inv = camera2ground.invert();
  Vector2i pixel(imageData_->image422.size.x() / 2, 0);
  imageSegments_->scanGrids[camera].clear();
  // Distance of the sample points in meter
  const float samplePointDistance = 0.02f;
  for (int y = 0; y < imageData_->image422.size.y(); y++)
  {
    pixel.y() = y;
    Vector2f robot(Vector2f::Zero());
    if (!cameraMatrix_->pixelToRobot(pixel, robot, camera2ground))
    {
      imageSegments_->scanGrids[camera].emplace_back(1, 2);
      continue;
    }
    Vector2i pixelX = Vector2i::Zero(), pixelY = Vector2i::Zero();
    if (!cameraMatrix_->robotToPixel({robot.x() - samplePointDistance, robot.y()}, pixelY,
                                     camera2ground_inv))
    {
      imageSegments_->scanGrids[camera].emplace_back(1, 2);
      continue;
    }
    if (!cameraMatrix_->robotToPixel({robot.x(), robot.y() - samplePointDistance}, pixelX,
                                     camera2ground_inv))
    {
      imageSegments_->scanGrids[camera].emplace_back(1, 2);
      continue;
    }
    imageSegments_->scanGrids[camera].emplace_back(std::max(pixelX.x() - pixel.x(), 1),
                                                   std::max(pixelY.y() - pixel.y(), 2));
  }
  scanGridsValid_[camera] =
      static_cast<int>(imageSegments_->scanGrids[camera].size()) == imageData_->image422.size.y();
}

void ImageSegmenter::addSegment(const Vector2i& peak, Scanline& scanline, EdgeType edgeType,
                                int scanPoints)
{
  ScanlineType& scanlineType = scanline.scanlineType;
  Segment& segment = scanline.segments.back();
  assert(peak.x() >= 0 && peak.y() >= 0);
  assert(scanline.scanlineType == ScanlineType::VERTICAL
             ? peak.y() < imageData_->image422.size.y()
             : peak.x() < imageData_->image422.size.x());
  assert(scanline.scanlineType == ScanlineType::VERTICAL ? peak.y() >= segment.start.y()
                                                         : peak.x() >= segment.start.x());
  segment.end = peak;
  segment.endEdgeType = edgeType;
  assert(scanPoints >= 0);
  segment.scanPoints = scanPoints;
  // TODO: determine color in another way for some segment lengths
  Vector2i diff = segment.end - segment.start;
  // To bit shift vectors coefficient wise
  auto shift = [](int c) { return c >> 1; };
  if ((scanlineType == ScanlineType::VERTICAL && diff.y() >= 6) || diff.x() >= 6)
  {
    Vector2i spacing = diff / 6;
    const YCbCr422& c1 = imageData_->image422.at(segment.start + spacing);
    const YCbCr422& c2 = imageData_->image422.at(segment.start + spacing * 2);
    const YCbCr422& c3 = imageData_->image422.at(segment.start + spacing * 3);
    const YCbCr422& c4 = imageData_->image422.at(segment.start + spacing * 4);
    const YCbCr422& c5 = imageData_->image422.at(segment.start + spacing * 5);
    segment.ycbcr422 = YCbCr422(Statistics::median(c1.y1_, c2.y1_, c3.y1_, c4.y1_, c5.y1_),
                                Statistics::median(c1.cb_, c2.cb_, c3.cb_, c4.cb_, c5.cb_),
                                Statistics::median(c1.y2_, c2.y2_, c3.y2_, c4.y2_, c5.y2_),
                                Statistics::median(c1.cr_, c2.cr_, c3.cr_, c4.cr_, c5.cr_));
  }
  else if (scanlineType == ScanlineType::VERTICAL && diff.y() > 2)
  {
    const YCbCr422& c1 = imageData_->image422.at(segment.start);
    const YCbCr422& c2 = imageData_->image422.at((segment.start + segment.end).unaryExpr(shift));
    const YCbCr422& c3 = imageData_->image422.at(segment.end);
    segment.ycbcr422 = YCbCr422(
        Statistics::median(c1.y1_, c2.y1_, c3.y1_), Statistics::median(c1.cb_, c2.cb_, c3.cb_),
        Statistics::median(c1.y2_, c2.y2_, c3.y2_), Statistics::median(c1.cr_, c2.cr_, c3.cr_));
  }
  else
  {
    segment.ycbcr422 = imageData_->image422.at((segment.start + segment.end).unaryExpr(shift));
  }
  segment.field = fieldColor_->isFieldColor(segment.ycbcr422);
  if (edgeType != EdgeType::BORDER && edgeType != EdgeType::END)
  {
    scanline.segments.emplace_back(peak, edgeType);
  }
}

template <bool useMedian>
void ImageSegmenter::createVerticalScanlines()
{
  // reinitialize scanlines if the image changes
  if (imageData_->image422.size != imageSegments_->imageSize || updateScanlines_)
  {
    updateScanlines_ = false;
    imageSegments_->init(imageData_->image422.size, numScanlines_());
  }

  ScanlineStateVertical scanlineState;
  std::vector<ScanlineStateVertical> scanlineStates;

  const int camera = static_cast<int>(imageData_->camera);
  const int edgeThreshold = edgeThresholdVertical_()[camera];

  scanlineState.gMin = edgeThreshold;
  scanlineState.gMax = -edgeThreshold;
  scanlineState.yPeak = 0;
  scanlineState.scanPoints = 0;
  scanlineStates.reserve(numScanlines_());


  int robotProjectionXMin = imageData_->image422.size.x();
  int robotProjectionXMax = 0, lineXMin = 0, lineXMax = 0;

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

  for (int i = 0; i < numScanlines_(); i++)
  {
    auto& scanline = imageSegments_->verticalScanlines[i];
    scanline.segments.emplace_back(Vector2i(scanline.pos, horizon), EdgeType::BORDER);

    // check if a robot part is visible in image, if so check it is part of the current scanline
    if (!robotProjection_->lines.empty() &&
        (robotProjectionXMin <= scanline.pos && robotProjectionXMax >= scanline.pos))
    {
      // search for the robotProjection line that intersects with the current scanline
      for (const auto& line : robotProjection_->lines)
      {
        lineXMin = std::min(line.p1.x(), line.p2.x());
        lineXMax = std::max(line.p1.x(), line.p2.x());
        if (lineXMin <= scanline.pos && lineXMax >= scanline.pos)
        {
          if (line.p1.x() == line.p2.x())
          {
            scanline.yMax =
                std::max(0, std::min(std::min(line.p1.y(), line.p2.y()), scanline.yMax));
          }
          else
          {
            scanline.yMax = std::max(0, std::min(line.getY(scanline.pos), scanline.yMax));
          }
        }
      }
    }
    scanlineState.lastYValue = imageData_->image422.at(horizon, scanline.pos).y1_;
    scanlineState.scanline = &scanline;
    scanlineStates.push_back(scanlineState);
  }
  const unsigned int sizeX = imageData_->image422.size.x();
  std::uint8_t yValue;
  const int upperBoundY =
      useMedian ? imageData_->image422.size.y() - 1 : imageData_->image422.size.y();
  for (int y = horizon + 2; y < upperBoundY; y += 2)
  {
    for (auto& state : scanlineStates)
    {
      if (y > state.scanline->yMax)
      {
        continue;
      }
      state.scanPoints++;
      if constexpr (useMedian)
      {
        const std::size_t PositionInArray =
            imageData_->image422.calculateCoordPositionInArray(y - 1, state.scanline->pos);
        yValue = Statistics::median(imageData_->image422.data[PositionInArray].y1_,
                                    imageData_->image422.data[PositionInArray + sizeX].y1_,
                                    imageData_->image422.data[PositionInArray + 2 * sizeX].y1_);
      }
      else
      {
        yValue = imageData_->image422.at(y, state.scanline->pos).y1_;
      }
      int diff = yValue - state.lastYValue;
      if (diff > state.gMax)
      {
        if (state.gMin < -edgeThreshold)
        {
          addSegment(Vector2i(state.scanline->pos, state.yPeak), (*state.scanline),
                     EdgeType::FALLING, state.scanPoints);
          state.scanPoints = 0;
        }
        state.gMax = diff;
        state.gMin = edgeThreshold;
        state.yPeak = y - 1;
      }
      if (diff < state.gMin)
      {
        if (state.gMax > edgeThreshold)
        {
          addSegment(Vector2i(state.scanline->pos, state.yPeak), (*state.scanline),
                     EdgeType::RISING, state.scanPoints);
          state.scanPoints = 0;
        }
        state.gMin = diff;
        state.gMax = -edgeThreshold;
        state.yPeak = y - 1;
      }
      state.lastYValue = yValue;
    }
  }
  // Add last segment of each scanline
  for (auto& vScanline : imageSegments_->verticalScanlines)
  {
    int scanPoints = vScanline.segments.size() >= 2
                         ? (vScanline.yMax - std::next(vScanline.segments.rbegin())->end.y())
                         : vScanline.yMax;
    scanPoints /= 2;
    if (vScanline.yMax > vScanline.segments.front().start.y())
    {
      addSegment(Vector2i(vScanline.pos, vScanline.yMax), vScanline, EdgeType::BORDER, scanPoints);
    }
    else
    {
      assert(vScanline.segments.size() == 1);
      vScanline.segments.clear();
    }
  }
}

bool ImageSegmenter::isRobotCheckNecessary(const int y) const
{
  for (auto& line : robotProjection_->lines)
  {
    if (line.p1.y() > y && line.p2.y() > y)
    {
      continue;
    }
    return true;
  }
  return false;
}

void ImageSegmenter::createHorizontalScanlines()
{
  // reinitialize scanlines if the image changes
  const int camera = static_cast<int>(imageData_->camera);
  if (imageData_->image422.size != imageSegments_->imageSize || !scanGridsValid_[camera])
  {
    calculateScanGrids();
  }
  if (!scanGridsValid_[camera])
  {
    return;
  }
  const int horizon =
      std::min(std::min(cameraMatrix_->getHorizonHeight(0),
                        cameraMatrix_->getHorizonHeight(imageData_->image422.size.x() - 1)),
               imageData_->image422.size.y() - 1);
  const int middleX = imageData_->image422.size.x() / 2;

  const int last = imageData_->image422.size.y() - 1;
  if (last == horizon)
  {
    return;
  }
  const int edgeThreshold = edgeThresholdHorizontal_()[camera];
  ScanlineStateHorizontal scanlineState;
  const std::vector<Vector2i>& scanGrid = imageSegments_->scanGrids[camera];
  Vector2i step = scanGrid[horizon];
  for (int y = horizon + 1; y < imageData_->image422.size.y(); y += step.y())
  {
    step = scanGrid[y];
    const int lookupX = imageSegments_->scanGrids[camera][y].x();
    HorizontalScanline scanline;
    scanline.segments.reserve(imageData_->image422.size.x());
    scanline.pos = y;
    scanline.step = step;
    bool wasOnRobot = false;
    int lastValidPoint = 0;
    Vector2i pixel(0, y);
    scanlineState.reset(edgeThreshold, &imageData_->image422[pixel]);
    // Don't move isRobotCheckNecessary function call to the if query. Its way slower!
    bool needsRobotCheck = isRobotCheckNecessary(y);
    int diff = 0;
    int x = 0;
    const YCbCr422* ycbcr422;
    const int startX = middleX % lookupX;
    for (x = startX; x < imageData_->image422.size.x(); x += lookupX)
    {
      pixel.x() = x;
      if (needsRobotCheck && robotProjection_->isOnRobot(pixel))
      {
        if (!wasOnRobot)
        {
          // if the current pixel is the first hit on the robot, end the segment.
          // TODO (pixel - step) would be more correct but requires extra handling at image
          // borders
          if (!scanline.segments.empty())
          {
            addSegment(pixel, scanline, EdgeType::END, scanlineState.scanPoints);
          }
        }
        wasOnRobot = true;
        continue;
      }
      lastValidPoint = x;
      if (scanline.segments.empty())
      {
        // first pixel that is not on a robot
        Vector2i startPixel = pixel;
        if (x == startX)
        {
          startPixel.x() = 0;
        }
        scanline.segments.emplace_back(startPixel,
                                       startPixel.x() == 0 ? EdgeType::BORDER : EdgeType::START);
        scanlineState.reset(edgeThreshold, &imageData_->image422[startPixel]);
        wasOnRobot = false;
        continue;
      }
      if (wasOnRobot)
      {
        // The previous sample point was the last one on the robot so start a new segment.
        scanline.segments.emplace_back(pixel, EdgeType::START);
        scanlineState.reset(edgeThreshold, &imageData_->image422[pixel]);
        wasOnRobot = false;
        continue;
      }
      scanlineState.scanPoints++;
      ycbcr422 = &imageData_->image422[pixel];
      // Symmetric gradient
      diff = ycbcr422->y1_ - scanlineState.lastYCbCr422->y1_;
      if (diff > scanlineState.gMax)
      {
        if (scanlineState.gMin < -edgeThreshold)
        {
          addSegment(Vector2i(scanlineState.xPeak, y), scanline, EdgeType::FALLING,
                     scanlineState.scanPoints);
          scanlineState.scanPoints = 0;
        }
        scanlineState.gMax = diff;
        scanlineState.gMin = edgeThreshold;
        // Correct the position of the edge (a little bit)
        scanlineState.xPeak = x - (lookupX / 2);
      }
      if (diff < scanlineState.gMin)
      {
        if (scanlineState.gMax > edgeThreshold)
        {
          addSegment(Vector2i(scanlineState.xPeak, y), scanline, EdgeType::RISING,
                     scanlineState.scanPoints);
          scanlineState.scanPoints = 0;
        }
        scanlineState.gMin = diff;
        scanlineState.gMax = -edgeThreshold;
        // Correct the position of the edge (a little bit)
        scanlineState.xPeak = x - (lookupX / 2);
      }
      scanlineState.lastYCbCr422 = ycbcr422;
    }
    if (!scanline.segments.empty())
    {
      // Add the last segment
      if (wasOnRobot)
      {
        addSegment(Vector2i(lastValidPoint, y), scanline, EdgeType::BORDER,
                   scanlineState.scanPoints);
      }
      else
      {
        addSegment(Vector2i(imageData_->image422.size.x() - 1, y), scanline, EdgeType::BORDER,
                   scanlineState.scanPoints);
      }
      imageSegments_->horizontalScanlines.push_back(scanline);
    }
  }
}

void ImageSegmenter::sendDebug()
{
  auto mount = mount_ + "." + imageData_->identification + "_vertical";
  if (debug().isSubscribed(mount))
  {
    Image vImage(imageData_->image422.get444From422Vector(imageData_->image422.size), Color::BLACK);
    if (drawFullImage_() && !imageSegments_->verticalScanlines.empty())
    {
      for (auto scanline = imageSegments_->verticalScanlines.begin();
           scanline != imageSegments_->verticalScanlines.end(); ++scanline)
      {
        for (auto& segment : scanline->segments)
        {
          for (int i = 0; i < (std::next(scanline)->pos - scanline->pos); ++i)
          {
            vImage.line(Image422::get444From422Vector(segment.start) + Vector2i(i, 0),
                        Image422::get444From422Vector(segment.end) + Vector2i(i, 0),
                        ColorConverter::colorFromYCbCr422(segment.ycbcr422));
          }
        }
      }
    }
    else
    {
      for (auto vScanline = imageSegments_->verticalScanlines.begin();
           vScanline != imageSegments_->verticalScanlines.end(); ++vScanline)
      {
        for (const auto& segment : vScanline->segments)
        {
          vImage.line(Image422::get444From422Vector(segment.start),
                      Image422::get444From422Vector(segment.end),
                      ColorConverter::colorFromYCbCr422(segment.ycbcr422));
          if (drawEdges_())
          {
            vImage.line(Image422::get444From422Vector(segment.start),
                        Image422::get444From422Vector(segment.start) +
                            Vector2i(std::next(vScanline)->pos - vScanline->pos, 0),
                        segment.startEdgeType == EdgeType::RISING
                            ? Color::RED
                            : segment.startEdgeType == EdgeType::FALLING ? Color::GREEN
                                                                         : Color::ORANGE);
            vImage.line(Image422::get444From422Vector(segment.end),
                        Image422::get444From422Vector(segment.end) +
                            Vector2i(std::next(vScanline)->pos - vScanline->pos, 0),
                        segment.startEdgeType == EdgeType::RISING
                            ? Color::RED
                            : segment.startEdgeType == EdgeType::FALLING ? Color::GREEN
                                                                         : Color::ORANGE);
          }
        }
      }
    }
    debug().sendImage(mount, vImage);
  }

  mount = mount_ + "." + imageData_->identification + "_horizontal";
  if (debug().isSubscribed(mount))
  {
    Image debugImage(imageData_->image422.get444From422Vector(imageData_->image422.size),
                     Color::BLACK);
    for (const auto& line : robotProjection_->lines)
    {
      Line<int> line444;
      line444.p1 = Image422::get444From422Vector(line.p1);
      line444.p2 = Image422::get444From422Vector(line.p2);
      debugImage.line(line444, Color::BLUE);
    }
    for (const auto& scanline : imageSegments_->horizontalScanlines)
    {
      for (const auto& segment : scanline.segments)
      {
        debugImage.line(Image422::get444From422Vector(segment.start),
                        Image422::get444From422Vector(segment.end),
                        ColorConverter::colorFromYCbCr422(segment.ycbcr422));
        if (drawEdges_())
        {
          debugImage.line(
              Image422::get444From422Vector(segment.start),
              Image422::get444From422Vector(segment.start) + Vector2i(0, scanline.step.y() - 1),
              segment.startEdgeType == EdgeType::RISING
                  ? Color::RED
                  : segment.startEdgeType == EdgeType::FALLING ? Color::GREEN : Color::ORANGE);
          debugImage.line(
              Image422::get444From422Vector(segment.end),
              Image422::get444From422Vector(segment.end) + Vector2i(0, scanline.step.y() - 1),
              segment.endEdgeType == EdgeType::RISING
                  ? Color::RED
                  : segment.endEdgeType == EdgeType::FALLING ? Color::GREEN : Color::ORANGE);
        }
      }
    }
    debug().sendImage(mount, debugImage);
  }

  mount = mount_ + "." + imageData_->identification + "_horizontal_grid";
  if (debug().isSubscribed(mount))
  {
    Image gridImage(imageData_->image422.to444Image());
    for (int y = 0; y < gridImage.size_.y(); ++y)
    {
      for (int x = 0; x < gridImage.size_.x(); ++x)
      {
        gridImage.at(y, x).cb_ = 128;
        gridImage.at(y, x).cr_ = 128;
      }
    }

    const int camera = static_cast<int>(imageData_->camera);
    if (scanGridsValid_[camera])
    {
      const int horizon =
          std::min(std::min(cameraMatrix_->getHorizonHeight(0),
                            cameraMatrix_->getHorizonHeight(imageData_->image422.size.x() - 1)),
                   imageData_->image422.size.y() - 1);
      const int last = imageData_->image422.size.y() - 1;
      if (last != horizon)
      {
        const int middleX = imageData_->image422.size.x() / 2;
        for (int y = horizon + 1; y < imageData_->image422.size.y();
             y += imageSegments_->scanGrids[camera][y].y())
        {
          const int lookupX = imageSegments_->scanGrids[camera][y].x();
          bool onRobotCheckNecessary = isRobotCheckNecessary(y);
          for (int x = middleX % lookupX; x < imageData_->image422.size.x(); x += lookupX)
          {
            const Vector2i pixel({x, y});
            const Vector2i pixel444 = Image422::get444From422Vector(pixel);
            if (imageData_->image422.isInside(pixel))
            {
              gridImage.circle(pixel444, 1,
                               onRobotCheckNecessary && robotProjection_->isOnRobot(pixel)
                                   ? Color::RED
                                   : Color::BLUE);
            }
          }
        }
        for (const auto& line : robotProjection_->lines)
        {
          Line<int> line444;
          line444.p1 = Image422::get444From422Vector(line.p1);
          line444.p2 = Image422::get444From422Vector(line.p2);
          gridImage.line(line444, Color::RED);
        }
      }
    }
    debug().sendImage(mount, gridImage);
  }
}
