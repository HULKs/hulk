#include <cmath>

#include "LineDetection.hpp"

#include "Tools/Chronometer.hpp"
#include "Tools/Math/Geometry.hpp"
#include "Tools/Math/Random.hpp"

LineDetection::LineDetection(const ModuleManagerInterface& manager)
  : Module(manager)
  , maxGapOnLine_(*this, "maxGapOnLine", [] {})
  , maxDistFromLine_(*this, "maxDistFromLine", [] {})
  , minNumberOfPointsOnLine_(*this, "minNumberOfPointsOnLine", [] {})
  , minPixelLength_(*this, "minPixelLength", [] {})
  , useDaylightFilter_(*this, "useDaylightFilter", [] {})
  , checkLineSegmentsProjection_(*this, "checkLineSegmentsProjection", [] {})
  , maxProjectedLineSegmentLength_(*this, "maxProjectedLineSegmentLength", [] {})
  , daylightThreshold_(*this, "daylightThreshold", [] {})
  , imageData_(*this)
  , cameraMatrix_(*this)
  , filteredSegments_(*this)
  , lineData_(*this)
{
}

Vector2f LineDetection::getGradient(const Vector2i& p) const
{
  std::function<uint8_t(const YCbCr422&)> y1 = [](const YCbCr422& c) { return c.y1_; };
  std::function<uint8_t(const YCbCr422&)> y2 = [](const YCbCr422& c) { return c.y2_; };
  const uint8_t one = 1;
  auto y = (p.x() & one) == 1 ? y2 : y1;
  Vector2f gradient = Vector2f::Zero();
  const Image422& im = imageData_->image422;
  if (p.x() < 1 || p.y() < 1 || p.x() + 1 >= im.size.x() || p.y() + 1 >= im.size.y())
  {
    return gradient;
  }
  gradient.x() = y(im.at(p.y() + 1, p.x() - 1)) + 2 * y(im.at(p.y() + 1, p.x())) +
                 y(im.at(p.y() + 1, p.x() + 1)) - y(im.at(p.y() - 1, p.x() - 1)) -
                 2 * y(im.at(p.y() - 1, p.x())) - y(im.at(p.y() - 1, p.x() + 1));
  gradient.y() = y(im.at(p.y() - 1, p.x() - 1)) + 2 * y(im.at(p.y(), p.x() - 1)) +
                 y(im.at(p.y() + 1, p.x() - 1)) - y(im.at(p.y() - 1, p.x() + 1)) -
                 2 * y(im.at(p.y(), p.x() + 1)) - y(im.at(p.y() + 1, p.x() + 1));
  return gradient.normalized();
}

void LineDetection::detectLinePoints()
{
  linePoints_.clear();
  Vector2f g1, g2;
  auto shift = [](int c) { return c >> 1; };
  for (const auto& segment : filteredSegments_->vertical)
  {
    // TODO: Maybe do seperate line detection for start and end points and then try to match them
    if (segment->startEdgeType != EdgeType::RISING || segment->endEdgeType != EdgeType::FALLING)
    {
      continue;
    }
    // TODO: Maybe move daylight filter after scalar product.
    if (useDaylightFilter_() && isIlluminated((segment->start + segment->end).unaryExpr(shift)))
    {
      continue;
    }
    if (checkLineSegmentsProjection_() && !hasReasonableSize(*segment))
    {
      continue;
    }
    g1 = getGradient(segment->start);
    g2 = getGradient(segment->end);
    if (g1.dot(g2) > -0.95)
    {
      continue;
    }
    // TODO: group line segments together according to their length
    linePoints_.push_back((segment->start + segment->end).unaryExpr(shift));
  }
}

bool LineDetection::hasReasonableSize(const Segment& segment) const
{
  Vector2f rStart, rEnd;
  const auto successfulProjection = [&](const Segment& segment) -> bool {
    return cameraMatrix_->pixelToRobot(segment.start, rStart) &&
           cameraMatrix_->pixelToRobot(segment.end, rEnd);
  };
  const auto reasonableSize = [&](const Vector2f& rStart, const Vector2f& rEnd) -> bool {
    return (rEnd - rStart).norm() <= maxProjectedLineSegmentLength_();
  };
  return successfulProjection(segment) && reasonableSize(rStart, rEnd);
}

bool LineDetection::isIlluminated(const Vector2i& p) const
{
  const float alpha = 0.333;
  const YCbCr422& data = imageData_->image422.at(p);
  int cr = data.cr_ - 128;
  int cb = data.cb_ - 128;
  int r = (data.y1_ + ((cr >> 2) + (cr >> 3) + (cr >> 5)));
  int g = (data.y1_ - ((cb >> 2) + (cb >> 4) + (cb >> 5)) -
           ((cr >> 1) + (cr >> 3) + (cr >> 4) + (cr >> 5)));
  int b = (data.y1_ + (cb + (cb >> 1) + (cb >> 2) + (cb >> 6)));
  double rd = static_cast<double>(r) / 255;
  double gd = static_cast<double>(g) / 255;
  double bd = static_cast<double>(b) / 255;
  double invariantY = 0.5 + std::log(gd / rd * std::pow(rd / bd, alpha));
  return invariantY > daylightThreshold_();
}

bool LineDetection::checkLength(const VecVector2i& linePoints) const
{
  assert(minNumberOfPointsOnLine_() >= 2); /// Otherwise the orthogonal projection can fail
  if ((static_cast<unsigned int>(linePoints.size()) < minNumberOfPointsOnLine_()) ||
      ((linePoints.front() - linePoints.back()).norm() < minPixelLength_()))
  {
    return false;
  }
  return true;
}

Vector2i LineDetection::getOrthogonalPixelProjection(const Vector2i& v, Line<int>& line)
{
  if (v == line.p1)
  {
    return line.p1;
  }
  else if (v == line.p2)
  {
    return line.p2;
  }
  else
  {
    Vector2i s = line.p2 - line.p1;
    assert(s.x() != 0 || s.y() != 0);
    float quotient = (v - line.p1).dot(s) / (float)(s.dot(s));
    float unshiftedX = s.x() * quotient;
    float unshiftedY = s.y() * quotient;
    return Vector2i(unshiftedX + line.p1.x(), unshiftedY + line.p1.y());
  }
}

void LineDetection::correctEndpoints(Line<int>& line, const VecVector2i& linePoints)
{
  assert(linePoints.front() != linePoints.back());
  line = Line<int>(getOrthogonalPixelProjection(linePoints.front(), line),
                   getOrthogonalPixelProjection(linePoints.back(), line));
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
  else
  {
    // TODO: check for the dimensions of the candidate
    lines_.push_back(detectedLine);
    return true;
  }
}

void LineDetection::ransacHandler()
{
  lines_.clear();
  VecVector2i best, unused;
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
                           int max_distance)
{
  bool valid = false;
  Line<int> line;
  int distance;
  const int sqr_max_distance = max_distance * max_distance;
  // Keep a buffer and a back-buffer, if we found the best line,
  // just swap the buffer and add to the other, until we find a better one.
  VecVector2<int> current_used_1, current_unused_1;
  VecVector2<int> current_used_2, current_unused_2;

  VecVector2<int>* current_used = &current_used_1;
  VecVector2<int>* current_unused = &current_unused_1;

  unsigned int max_score = 0;
  if (points.size() < 2)
  {
    best.clear();
    unused = points;
    return valid;
  }

  current_used_1.reserve(points.size());
  current_used_2.reserve(points.size());
  current_unused_1.reserve(points.size());
  current_unused_2.reserve(points.size());

  for (unsigned int i = 0; i < iterations; i++)
  {
    line.p1 = points[Random::uniformInt(0, points.size() - 1)];
    line.p2 = points[Random::uniformInt(0, points.size() - 1)];

    if (line.p1 == line.p2)
    {
      continue;
    }
    current_used->clear();
    current_unused->clear();

    for (const auto& point : points)
    {
      distance = Geometry::getSquaredLineDistance(line, point);
      assert(distance >= 0);

      if (distance <= sqr_max_distance)
      {
        current_used->push_back(point);
      }
      else
      {
        current_unused->push_back(point);
      }
    }

    if (current_used->size() > max_score)
    {
      max_score = current_used->size();
      bestLine = line;
      if (current_used == &current_used_1)
      {
        current_used = &current_used_2;
        current_unused = &current_unused_2;
      }
      else
      {
        current_used = &current_used_1;
        current_unused = &current_unused_1;
      }
    }
  }

  best = current_used == &current_used_1 ? current_used_2 : current_used_1;
  unused = current_unused == &current_unused_1 ? current_unused_2 : current_unused_1;

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
    Vector2f start, end;
    if (!cameraMatrix_->pixelToRobot(line.p1, start) || !cameraMatrix_->pixelToRobot(line.p2, end))
    {
      continue;
    }

    lineData_->lines.emplace_back(start, end);
    auto& newLine = lineData_->lines[lineId];
    lineData_->lineInfos.emplace_back(newLine,
                                      Geometry::getLineSegmentDistance(newLine, {0.f, 0.f}),
                                      (newLine.p1 - newLine.p2).norm(), lineId);
    ++lineId;
  }
  lineData_->timestamp = imageData_->timestamp;
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
      image.circle(Image422::get444From422Vector(point), 2, Color::RED);
    }
    for (const auto& line : lines_)
    {
      image.line(Image422::get444From422Vector(line.p1), Image422::get444From422Vector(line.p2),
                 Color::BLUE);
    }
    debug().sendImage(mount_ + "." + imageData_->identification + "_image_lines", image);
  }
}
