#include <cmath>

#include "FieldMarksDetection.hpp"

#include "Utils/Algorithms.hpp"
#include "print.hpp"

#include "Tools/Chronometer.hpp"

FieldMarksDetection::FieldMarksDetection(const ModuleManagerInterface& manager)
  : Module(manager, "FieldMarksDetection")
  , maxGapOnLine_(*this, "maxGapOnLine", [] {})
  , maxDistFromLine_(*this, "maxDistFromLine", [] {})
  , minNumberOfPointsOnLine_(*this, "minNumberOfPointsOnLine", [] {})
  , minPixelLength_(*this, "minPixelLength", [] {})
  , useDaylightFilter_(*this, "useDaylightFilter", [] {})
  , daylightThreshold_(*this, "daylightThreshold", [] {})
  , image_data_(*this)
  , camera_matrix_(*this)
  , filtered_regions_(*this)
  , line_data_(*this)
  , circle_data_(*this)
{
}

Vector2f FieldMarksDetection::getGradient(const Vector2i& p) const
{
  Vector2f gradient;
  const Image& im = image_data_->image;
  if (p.x() < 1 || p.y() < 1 || p.x() + 1 >= im.size_.x() || p.y() + 1 >= im.size_.y())
  {
    return gradient;
  }
  gradient.x() = im.at(p.y() + 1, p.x() - 1).y_ + 2 * im.at(p.y() + 1, p.x()).y_ + im.at(p.y() + 1, p.x() + 1).y_ - im.at(p.y() - 1, p.x() - 1).y_ -
                 2 * im.at(p.y() - 1, p.x()).y_ - im.at(p.y() - 1, p.x() + 1).y_;
  gradient.y() = im.at(p.y() - 1, p.x() - 1).y_ + 2 * im.at(p.y(), p.x() - 1).y_ + im.at(p.y() + 1, p.x() - 1).y_ - im.at(p.y() - 1, p.x() + 1).y_ -
                 2 * im.at(p.y(), p.x() + 1).y_ - im.at(p.y() + 1, p.x() + 1).y_;
  return gradient.normalized();
}

void FieldMarksDetection::detectLinePoints()
{
  line_points_.clear();
  Vector2f g1, g2;
  for (auto& it : filtered_regions_->scanlines)
  {
    for (auto& it2 : it.regions)
    {
      // TODO: Maybe do seperate line detection for start and end points and then try to match them
      if (it2.start_edge != EdgeType::RISING || it2.end_edge != EdgeType::FALLING)
      {
        continue;
      }
      // TODO: Maybe move daylight filter after scalar product.
      if (useDaylightFilter_() && isIlluminated(it.x, (it2.start + it2.end) / 2))
      {
        continue;
      }
      g1 = getGradient(Vector2i(it.x, it2.start));
      g2 = getGradient(Vector2i(it.x, it2.end));
      if (g1.dot(g2) > -0.95)
      {
        continue;
      }
      // TODO: group line segments together according to their length
      line_points_.emplace_back(it.x, (it2.start + it2.end) / 2);
    }
  }
}

bool FieldMarksDetection::isIlluminated(const unsigned int x, const unsigned int y) const
{
  const float alpha = 0.333;
  const Color data = image_data_->image.at(y, x);
  int Cr = data.cr_ - 128;
  int Cb = data.cb_ - 128;
  int R = (data.y_ + ((Cr >> 2) + (Cr >> 3) + (Cr >> 5)));
  int G = (data.y_ - ((Cb >> 2) + (Cb >> 4) + (Cb >> 5)) - ((Cr >> 1) + (Cr >> 3) + (Cr >> 4) + (Cr >> 5)));
  int B = (data.y_ + (Cb + (Cb >> 1) + (Cb >> 2) + (Cb >> 6)));
  double Rd = static_cast<double>(R) / 255;
  double Gd = static_cast<double>(G) / 255;
  double Bd = static_cast<double>(B) / 255;
  double invariant_y = 0.5 + std::log(Gd / Rd * std::pow(Rd / Bd, alpha));
  return invariant_y > daylightThreshold_();
}

bool FieldMarksDetection::checkLength(const VecVector2i& linePoints) const
{
  assert(minNumberOfPointsOnLine_() >= 2); /// Otherwise the orthogonal projection can fail
  if ((static_cast<unsigned int>(linePoints.size()) < minNumberOfPointsOnLine_()) || ((linePoints.front() - linePoints.back()).norm() < minPixelLength_()))
  {
    return false;
  }
  return true;
}

Vector2i FieldMarksDetection::getOrthogonalPixelProjection(const Vector2i& v, Line<int>& line)
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

void FieldMarksDetection::correctEndpoints(Line<int>& line, const VecVector2i& linePoints)
{
  assert(linePoints.front() != linePoints.back());
  line = Line<int>(getOrthogonalPixelProjection(linePoints.front(), line), getOrthogonalPixelProjection(linePoints.back(), line));
  /// Always ensure the point order when working with our line detection
  if (line.p1.x() > line.p2.x())
  {
    Vector2i swap = line.p1;
    line.p1 = line.p2;
    line.p2 = swap;
  }
  assert(line.p1.x() <= line.p2.x());
}

bool FieldMarksDetection::correctLine(Line<int> detectedLine, VecVector2i& linePoints, VecVector2i& unusedPoints)
{
  std::sort(linePoints.begin(), linePoints.end(), [](const Vector2i& p1, const Vector2i& p2) { return (p1.x() < p2.x()); });
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

void FieldMarksDetection::ransac()
{
  lines_.clear();
  VecVector2i best, unused;
  for (unsigned int i = 0; i < 5 && line_points_.size() > 5; i++)
  {
    /// decreasing ransac iterations since unused getting smaller
    Line<int> line;
    if (Algorithms::ransacLine(line, line_points_, best, unused, 20 - 4 * i, maxDistFromLine_()))
    {
      correctLine(line, best, unused);
      line_points_ = unused;
    }
    else
    {
      --i;
    }
  }
}

void FieldMarksDetection::createLineData()
{
  unsigned int index = 0;
  for (auto& it : lines_)
  {
    Vector2f start, end;
    if (!camera_matrix_->pixelToRobot(it.p1, start) || !camera_matrix_->pixelToRobot(it.p2, end))
    {
      continue;
    }
    line_data_->vertices.push_back(start);
    line_data_->vertices.push_back(end);
    line_data_->edges.emplace_back(index, index + 1);
    index += 2;
  }
  line_data_->timestamp = image_data_->timestamp;

  debug().update(mount_ + ".LineData", *line_data_);
}

void FieldMarksDetection::cycle()
{
  if (!filtered_regions_->valid)
  {
    return;
  }
  VecVector2i saved_line_points;
  {
    Chronometer time(debug(), mount_ + "." + image_data_->identification + "_cycle_time");
    detectLinePoints();
    saved_line_points = line_points_;
    ransac();
    createLineData();
  }
  if (debug().isSubscribed(mount_ + "." + image_data_->identification + "_image"))
  {
    Image image(image_data_->image);
    for (auto& it : saved_line_points)
    {
      image.circle(it, 2, Color::RED);
    }
    for (auto& it : lines_)
    {
      image.line(it.p1, it.p2, Color::BLUE);
    }
    debug().sendImage(mount_ + "." + image_data_->identification + "_image", image);
  }
}
