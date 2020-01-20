#include "FieldBorderDetection.hpp"

#include "Tools/Chronometer.hpp"
#include "Tools/Math/ColorConverter.hpp"
#include "Tools/Math/Geometry.hpp"
#include "Tools/Math/Random.hpp"
#include "Tools/Storage/Image.hpp"

#include "Definitions/windows_definition_fix.hpp"

FieldBorderDetection::FieldBorderDetection(const ModuleManagerInterface& manager)
  : Module(manager)
  , angleThreshold_(*this, "angleThreshold", [] {})
  , minPointsPerLine_(*this, "minPointsPerLine", [] {})
  , drawVerticalFilteredSegments_(*this, "drawVerticalFilteredSegments", [] {})
  , drawHorizontalFilteredSegments_(*this, "drawHorizontalFilteredSegments", [] {})
  , drawVerticalEdges_(*this, "drawVerticalEdges", [] {})
  , drawHorizontalEdges_(*this, "drawHorizontalEdges", [] {})
  , imageData_(*this)
  , imageSegments_(*this)
  , cameraMatrix_(*this)
  , fieldBorder_(*this)
  , filteredSegments_(*this)
{
}

bool FieldBorderDetection::isOrthogonal(const Line<int>& l1, const Line<int>& l2)
{
  Vector2f l1_start, l1_end, l2_start, l2_end;
  cameraMatrix_->pixelToRobot(l1.p1, l1_start);
  cameraMatrix_->pixelToRobot(l2.p1, l2_start);
  cameraMatrix_->pixelToRobot(l1.p2, l1_end);
  cameraMatrix_->pixelToRobot(l2.p2, l2_end);

  Vector2f vec1 = l1_end - l1_start;
  Vector2f vec2 = l2_end - l2_start;

  float angle = std::acos(vec1.normalized().dot(vec2.normalized()));
  float angleInDeg = angle / TO_RAD;
  debug().update(mount_ + ".AngleInDeg", angleInDeg);
  debug().update(mount_ + ".AngleInRad", angle);
  if (angle > (M_PI_2 - angleThreshold_() * TO_RAD) &&
      angle < (M_PI_2 + angleThreshold_() * TO_RAD) && (vec1.x() != 0 || vec1.y() != 0))
  {
    return true;
  }
  else
  {
    return false;
  }
}

Vector2i FieldBorderDetection::centerOfGroup(VecVector2i group)
{
  Vector2i center(0, 0);
  for (const auto& point : group)
  {
    center.x() += point.x();
    center.y() += point.y();
  }
  center.x() = (center.x() / group.size());
  center.y() = (center.y() / group.size());
  return center;
}

Line<int> FieldBorderDetection::bestFitLine(VecVector2i points)
{
  // Divide points into two equal groups (sorted from left to right)
  std::size_t const half_size = points.size() / 2;
  VecVector2i leftGroup(points.begin(), points.begin() + half_size);
  VecVector2i rightGroup(points.begin() + half_size, points.end());
  // Evaluate the center off both groups
  Vector2i leftCenter = centerOfGroup(leftGroup);
  Vector2i rightCenter = centerOfGroup(rightGroup);
  // Return line through both centers
  return Line<int>(leftCenter, rightCenter);
}

void FieldBorderDetection::findBorderPoints()
{
  for (const auto& vScanline : imageSegments_->verticalScanlines)
  {
    for (const auto& segment : vScanline.segments)
    {
      if (segment.field >= 0.5f)
      {
        borderPoints_.push_back(segment.start);
        break;
      }
    }
  }
}

void FieldBorderDetection::findBorderLines()
{
  VecVector2i firstLinePoints, secondLinePoints, firstUnusedPoints, secondUnusedPoints;
  // Find points for the first line
  Line<int> line; // temporary dummy line
  if (!ransac(line, borderPoints_, firstLinePoints, firstUnusedPoints, 20, 2))
  {
    return;
  }

  if (static_cast<int>(firstLinePoints.size()) >= minPointsPerLine_())
  {
    // Accept line
    Line<int> first;
    first = bestFitLine(firstLinePoints);
    fieldBorder_->borderLines.push_back(first);

    // If enough points are left, check if second line exists
    if (static_cast<int>(firstUnusedPoints.size()) >= minPointsPerLine_() &&
        ransac(line, firstUnusedPoints, secondLinePoints, secondUnusedPoints, 20, 4))
    {
      if (static_cast<int>(secondLinePoints.size()) >= minPointsPerLine_())
      {
        Line<int> second;
        second = bestFitLine(secondLinePoints);
        // Check if second line is orthogonal to first
        if (isOrthogonal(first, second))
        {
          // Accept line
          fieldBorder_->borderLines.push_back(second);
          // Two have been found
        }
        else
        {
          // Only one line has been found
        }
      }
    }
  }
  else
  {
    // Only one line has been found
  }
}

bool FieldBorderDetection::ransac(Line<int>& bestLine, const VecVector2<int>& points,
                                  VecVector2<int>& best, VecVector2<int>& unused,
                                  unsigned int iterations, int max_distance)
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

void FieldBorderDetection::createFilteredSegments()
{
  for (const auto& scanline : imageSegments_->verticalScanlines)
  {
    bool below = false;
    for (const auto& segment : scanline.segments)
    {
      if (!below && fieldBorder_->isInsideField(segment.start))
      {
        below = true;
      }
      if (below && segment.field < 0.5f)
      {
        filteredSegments_->vertical.push_back(&segment);
      }
    }
  }
  for (const auto& scanline : imageSegments_->horizontalScanlines)
  {
    bool foundField = false;
    bool noOtherInterestingSegments = false;
    for (const auto& segment : scanline.segments)
    {
      if (noOtherInterestingSegments)
      {
        break;
      }
      const bool insideField = fieldBorder_->isInsideField((segment.start)) &&
                               fieldBorder_->isInsideField((segment.end));
      if (!foundField && insideField)
      {
        foundField = true;
      }
      if (foundField)
      {
        if (!insideField)
        {
          noOtherInterestingSegments = true;
          break;
        }
        if (segment.field < 0.5f)
        {
          filteredSegments_->horizontal.push_back(&segment);
        }
      }
    }
  }
  filteredSegments_->valid = true;
}

void FieldBorderDetection::cycle()
{
  if (!imageSegments_->valid)
  {
    return;
  }
  {
    Chronometer time(debug(), mount_ + ".cycle_time");
    // Reset private members
    borderPoints_.clear();
    fieldBorder_->imageSize = imageData_->image422.size;
    // Find border points
    findBorderPoints();
    // Find the border lines
    findBorderLines();
    createFilteredSegments();
  }
  sendImagesForDebug();
}

void FieldBorderDetection::sendImagesForDebug()
{
  auto mount = mount_ + "." + imageData_->identification + "_image";
  if (debug().isSubscribed(mount))
  {
    Image fieldBorderImage(imageData_->image422.to444Image());

    for (const auto& bp : borderPoints_)
    {
      fieldBorderImage.circle(Image422::get444From422Vector(bp), 3, Color::BLACK);
    }

    VecVector2i allBorderPoints = fieldBorder_->getBorderPoints();
    for (const auto& bp : allBorderPoints)
    {
      fieldBorderImage[Image422::get444From422Vector(bp)] = Color::BLUE;
    }
    for (const auto& line : fieldBorder_->borderLines)
    {
      Line<int> line444;
      line444.p1 = Image422::get444From422Vector(line.p1);
      line444.p2 = Image422::get444From422Vector(line.p2);
      fieldBorderImage.line(line444, Color::RED);
      fieldBorderImage.line(Vector2i(line444.p1.x(), line444.p1.y() + 1),
                            Vector2i(line444.p2.x(), line444.p2.y() + 1), Color::RED);
      fieldBorderImage.line(Vector2i(line444.p1.x(), line444.p1.y() + 1),
                            Vector2i(line444.p2.x(), line444.p2.y() + 1), Color::RED);
      fieldBorderImage.line(Vector2i(line444.p1.x(), line444.p1.y() - 1),
                            Vector2i(line444.p2.x(), line444.p2.y() - 1), Color::RED);
    }
    debug().sendImage(mount_ + "." + imageData_->identification + "_image", fieldBorderImage);
  }

  mount = mount_ + "." + imageData_->identification + "_filtered";
  if (debug().isSubscribed(mount))
  {
    if (imageSegments_->verticalScanlines.empty())
    {
      return;
    }
    Image image(imageData_->image422.get444From422Vector(imageData_->image422.size), Color::BLACK);
    for (const auto& segment : filteredSegments_->vertical)
    {
      if (drawVerticalFilteredSegments_())
      {

        image.line(Image422::get444From422Vector(segment->start),
                   Image422::get444From422Vector(segment->end),
                   ColorConverter::colorFromYCbCr422(segment->ycbcr422));
      }
      if (drawVerticalEdges_())
      {
        image.line(Image422::get444From422Vector(segment->start),
                   Image422::get444From422Vector(segment->start) + Vector2i(2, 0),
                   segment->startEdgeType == EdgeType::RISING
                       ? Color::RED
                       : segment->startEdgeType == EdgeType::FALLING ? Color::GREEN
                                                                     : Color::ORANGE);
        image.line(Image422::get444From422Vector(segment->end),
                   Image422::get444From422Vector(segment->end) + Vector2i(2, 0),
                   segment->endEdgeType == EdgeType::RISING
                       ? Color::RED
                       : segment->endEdgeType == EdgeType::FALLING ? Color::GREEN : Color::ORANGE);
      }
    }
    for (const auto& segment : filteredSegments_->horizontal)
    {
      if (drawHorizontalFilteredSegments_())
      {

        image.line(Image422::get444From422Vector(segment->start),
                   Image422::get444From422Vector(segment->end),
                   ColorConverter::colorFromYCbCr422(segment->ycbcr422));
      }
      if (drawHorizontalEdges_())
      {
        image.line(Image422::get444From422Vector(segment->start),
                   Image422::get444From422Vector(segment->start) + Vector2i(0, 2),
                   segment->startEdgeType == EdgeType::RISING
                       ? Color::RED
                       : segment->startEdgeType == EdgeType::FALLING ? Color::GREEN
                                                                     : Color::ORANGE);
        image.line(Image422::get444From422Vector(segment->end),
                   Image422::get444From422Vector(segment->end) + Vector2i(0, 2),
                   segment->endEdgeType == EdgeType::RISING
                       ? Color::RED
                       : segment->endEdgeType == EdgeType::FALLING ? Color::GREEN : Color::ORANGE);
      }
    }
    debug().sendImage(mount, image);
  }
}
