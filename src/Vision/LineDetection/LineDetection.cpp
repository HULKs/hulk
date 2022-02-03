#include <cmath>

#include "Vision/LineDetection/LineDetection.hpp"

#include "Tools/Chronometer.hpp"
#include "Tools/Math/Geometry.hpp"
#include "Tools/Math/Random.hpp"

LineDetection::LineDetection(const ModuleManagerInterface& manager)
  : Module(manager)
  , maxGapOnLine_(*this, "maxGapOnLine", [] {})
  , maxDistFromLine_(*this, "maxDistFromLine", [] {})
  , minNumberOfPointsOnLine_(*this, "minNumberOfPointsOnLine", [] {})
  , minPixelLength_(*this, "minPixelLength", [] {})
  , checkLineSegmentsProjection_(*this, "checkLineSegmentsProjection", [] {})
  , maxProjectedLineSegmentLength_(*this, "maxProjectedLineSegmentLength", [] {})
  , imageData_(*this)
  , cameraMatrix_(*this)
  , filteredSegments_(*this)
  , lineData_(*this)
{
}

Vector2f LineDetection::getGradient(const Vector2i& p) const
{
  const auto y1 = [](const YCbCr422& c) { return c.y1; };
  const auto y2 = [](const YCbCr422& c) { return c.y2; };
  const auto y = (static_cast<unsigned int>(p.x()) & 1u) == 1 ? y2 : y1;
  Vector2f gradient = Vector2f::Zero();
  const Image422& im = imageData_->image422;
  if (p.x() < 1 || p.y() < 1 || p.x() + 1 >= im.size.x() || p.y() + 1 >= im.size.y())
  {
    return gradient;
  }
  // -1 -2 -1
  //  0  0  0
  //  1  2  1
  gradient.x() = y(im.at(p.y() + 1, p.x() - 1)) + 2 * y(im.at(p.y() + 1, p.x())) +
                 y(im.at(p.y() + 1, p.x() + 1)) - y(im.at(p.y() - 1, p.x() - 1)) -
                 2 * y(im.at(p.y() - 1, p.x())) - y(im.at(p.y() - 1, p.x() + 1));
  //  1  0  -1
  //  2  0  -2
  //  1  0  -1
  gradient.y() = y(im.at(p.y() - 1, p.x() - 1)) + 2 * y(im.at(p.y(), p.x() - 1)) +
                 y(im.at(p.y() + 1, p.x() - 1)) - y(im.at(p.y() - 1, p.x() + 1)) -
                 2 * y(im.at(p.y(), p.x() + 1)) - y(im.at(p.y() + 1, p.x() + 1));
  return gradient.normalized();
}

void LineDetection::detectLinePoints()
{
  linePoints_.clear();
  Vector2f gradientStart;
  Vector2f gradientEnd;
  for (auto segmentIterator = filteredSegments_->vertical.begin();
       segmentIterator != filteredSegments_->vertical.end(); ++segmentIterator)
  {
    // TODO: Maybe do seperate line detection for start and end points and then try to match them
    if ((*segmentIterator)->startEdgeType != EdgeType::RISING ||
        (*segmentIterator)->endEdgeType != EdgeType::FALLING)
    {
      continue;
    }
    if (checkLineSegmentsProjection_() && !hasReasonableSize(**segmentIterator))
    {
      continue;
    }
    gradientStart = getGradient((*segmentIterator)->start);
    gradientEnd = getGradient((*segmentIterator)->end);
    // to save computation of an arccos() to determine the angle between the gradient vectors
    // As cos(180°) = -1, one can check that the vectors are facing in opposite directions
    // by comparing the dot product with -0.95 = cos(161.805°)
    constexpr float cos161 = -0.95f;
    if (gradientStart.dot(gradientEnd) > cos161)
    {
      continue;
    }
    linePoints_.emplace_back(((*segmentIterator)->start + (*segmentIterator)->end) / 2);
    lineData_->usedVerticalFilteredSegments[std::distance(filteredSegments_->vertical.begin(),
                                                          segmentIterator)] = true;
  }
}

bool LineDetection::hasReasonableSize(const Segment& segment) const
{
  const std::optional<Vector2f> rStart = cameraMatrix_->pixelToRobot(segment.start);
  const std::optional<Vector2f> rEnd = cameraMatrix_->pixelToRobot(segment.end);
  if (!rStart.has_value() || !rEnd.has_value())
  {
    return false;
  }
  const auto reasonableSize = (*rEnd - *rStart).norm() <= maxProjectedLineSegmentLength_();
  return reasonableSize;
}

bool LineDetection::checkLength(const VecVector2i& linePoints) const
{
  assert(minNumberOfPointsOnLine_() >= 2); /// Otherwise the orthogonal projection can fail
  const int lineLength = (linePoints.front() - linePoints.back()).norm();
  return linePoints.size() >= minNumberOfPointsOnLine_() && lineLength >= minPixelLength_();
}

void LineDetection::correctEndpoints(Line<int>& line, const VecVector2i& linePoints)
{
  assert(linePoints.front() != linePoints.back());
  line = Line<int>(Geometry::projectPointOnLine(linePoints.front(), line),
                   Geometry::projectPointOnLine(linePoints.back(), line));
  /// Always ensure the point order when working with our line detection
  if (line.p1.x() > line.p2.x())
  {
    Vector2i swap = line.p1;
    line.p1 = line.p2;
    line.p2 = swap;
  }
  assert(line.p1.x() <= line.p2.x());
}

bool LineDetection::correctLine(Line<int> detectedLine, VecVector2i& linePoints,
                                VecVector2i& unusedPoints)
{
  std::sort(linePoints.begin(), linePoints.end(),
            [](const Vector2i& p1, const Vector2i& p2) { return (p1.x() < p2.x()); });
  assert(linePoints.front().x() <= linePoints.back().x());
  if (!checkLength(linePoints))
  {
    return false;
  }
  correctEndpoints(detectedLine, linePoints);
  auto it = linePoints.begin();
  for (; std::next(it) != linePoints.end(); it++)
  {
    float distance = ((*it) - (*std::next(it))).norm();
    if (distance > maxGapOnLine_())
    {
      break;
    }
  }
  if (std::next(it) != linePoints.end())
  {
    Line<int> l1(linePoints.front(), *it);
    Line<int> l2(*std::next(it), linePoints.back());
    VecVector2i v1(linePoints.begin(), std::next(it));
    VecVector2i v2(std::next(it), linePoints.end());
    if (checkLength(v1))
    {
      correctEndpoints(l1, v1);
      lines_.push_back(l1);
    }
    else
    {
      unusedPoints.insert(unusedPoints.end(), v1.begin(), v1.end());
    }
    return correctLine(l2, v2, unusedPoints);
  }
  lines_.emplace_back(detectedLine);
  return true;
}

void LineDetection::ransacHandler()
{
  lines_.clear();
  VecVector2i best;
  VecVector2i unused;
  for (unsigned int i = 0; i < 5 && linePoints_.size() > 5; i++)
  {
    /// decreasing ransac iterations since unused getting smaller
    Line<int> line;
    if (ransac(line, linePoints_, best, unused, 20 - 4 * i, maxDistFromLine_()))
    {
      correctLine(line, best, unused);
      linePoints_ = unused;
    }
    else
    {
      --i;
    }
  }
}

bool LineDetection::ransac(Line<int>& bestLine, const VecVector2<int>& points,
                           VecVector2<int>& best, VecVector2<int>& unused, unsigned int iterations,
                           int maxDistance)
{
  bool valid = false;
  Line<int> line;
  int distance;
  const int sqrMaxDistance = maxDistance * maxDistance;
  // Keep a buffer and a back-buffer, if we found the best line,
  // just swap the buffer and add to the other, until we find a better one.
  VecVector2<int> currentUsed1;
  VecVector2<int> currentUnused1;
  VecVector2<int> currentUsed2;
  VecVector2<int> currentUnused2;

  VecVector2<int>* currentUsed = &currentUsed1;
  VecVector2<int>* currentUnused = &currentUnused1;

  unsigned int maxScore = 0;
  if (points.size() < 2)
  {
    best.clear();
    unused = points;
    return valid;
  }

  currentUsed1.reserve(points.size());
  currentUsed2.reserve(points.size());
  currentUnused1.reserve(points.size());
  currentUnused2.reserve(points.size());

  for (unsigned int i = 0; i < iterations; i++)
  {
    line.p1 = points[Random::uniformInt(0, points.size() - 1)];
    line.p2 = points[Random::uniformInt(0, points.size() - 1)];

    if (line.p1 == line.p2)
    {
      continue;
    }
    currentUsed->clear();
    currentUnused->clear();

    for (const auto& point : points)
    {
      distance = Geometry::getSquaredLineDistance(line, point);
      assert(distance >= 0);

      if (distance <= sqrMaxDistance)
      {
        currentUsed->push_back(point);
      }
      else
      {
        currentUnused->push_back(point);
      }
    }

    if (currentUsed->size() > maxScore)
    {
      maxScore = currentUsed->size();
      bestLine = line;
      if (currentUsed == &currentUsed1)
      {
        currentUsed = &currentUsed2;
        currentUnused = &currentUnused2;
      }
      else
      {
        currentUsed = &currentUsed1;
        currentUnused = &currentUnused1;
      }
    }
  }

  best = currentUsed == &currentUsed1 ? currentUsed2 : currentUsed1;
  unused = currentUnused == &currentUnused1 ? currentUnused2 : currentUnused1;

  if (best.empty() || bestLine.p1 == bestLine.p2)
  {
    best.clear();
    unused = points;
    return valid;
  }

  valid = true;
  return valid;
}

void LineDetection::createLineData()
{
  lineData_->lines.reserve(lines_.size());
  lineData_->lineInfos.reserve(lines_.size());

  unsigned int lineId = 0;
  for (const auto& line : lines_)
  {
    const std::optional<Vector2f> start = cameraMatrix_->pixelToRobot(line.p1);
    const std::optional<Vector2f> end = cameraMatrix_->pixelToRobot(line.p2);
    if (!start.has_value() || !end.has_value())
    {
      continue;
    }

    lineData_->lines.emplace_back(start.value(), end.value());
    auto& newLine = lineData_->lines[lineId];
    lineData_->lineInfos.emplace_back(newLine,
                                      Geometry::getLineSegmentDistance(newLine, {0.f, 0.f}),
                                      (newLine.p1 - newLine.p2).norm(), lineId);
    ++lineId;
  }
  lineData_->timestamp = imageData_->captureTimePoint;
  lineData_->valid = true;

  debug().update(mount_ + ".LineData", *lineData_);
}

void LineDetection::cycle()
{
  if (!filteredSegments_->valid)
  {
    return;
  }
  {
    Chronometer time(debug(), mount_ + "." + imageData_->identification + "_cycle_time");
    lineData_->usedVerticalFilteredSegments.clear();
    lineData_->usedVerticalFilteredSegments.resize(filteredSegments_->vertical.size(), false);
    detectLinePoints();
    debugLinePoints_ = linePoints_;
    ransacHandler();
    createLineData();
  }
  sendImagesForDebug();
}

void LineDetection::sendImagesForDebug()
{
  auto mount = mount_ + "." + imageData_->identification + "_image_lines";
  if (debug().isSubscribed(mount))
  {
    Image image(imageData_->image422.to444Image());
    for (const auto& point : debugLinePoints_)
    {
      image.drawCircle(Image422::get444From422Vector(point), 2, Color::RED);
    }
    for (const auto& line : lines_)
    {
      image.drawLine(Image422::get444From422Vector(line.p1), Image422::get444From422Vector(line.p2),
                     Color::BLUE);
    }
    debug().sendImage(mount_ + "." + imageData_->identification + "_image_lines", image);
  }
}
