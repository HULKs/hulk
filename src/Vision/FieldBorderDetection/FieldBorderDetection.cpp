#include "Vision/FieldBorderDetection/FieldBorderDetection.hpp"

#include "Tools/Chronometer.hpp"
#include "Tools/Math/Geometry.hpp"
#include "Tools/Math/Random.hpp"
#include "Tools/Storage/Image.hpp"

FieldBorderDetection::FieldBorderDetection(const ModuleManagerInterface& manager)
  : Module(manager)
  , angleThreshold_(*this, "angleThreshold", [] {})
  , minPointsPerLine_(*this, "minPointsPerLine", [] {})
  , maxBorderPointsNotEnclosed_(*this, "maxBorderPointsNotEnclosed", [] {})
  , drawVerticalFilteredSegments_(*this, "drawVerticalFilteredSegments", [] {})
  , drawHorizontalFilteredSegments_(*this, "drawHorizontalFilteredSegments", [] {})
  , drawVerticalEdges_(*this, "drawVerticalEdges", [] {})
  , drawHorizontalEdges_(*this, "drawHorizontalEdges", [] {})
  , imageData_(*this)
  , imageSegments_(*this)
  , cameraMatrix_(*this)
  , fieldBorder_(*this)
{
}

bool FieldBorderDetection::isOrthogonal(const Line<int>& l1, const Line<int>& l2)
{
  const std::optional<Vector2f> l1Start = cameraMatrix_->pixelToRobot(l1.p1);
  const std::optional<Vector2f> l1End = cameraMatrix_->pixelToRobot(l1.p2);
  const std::optional<Vector2f> l2Start = cameraMatrix_->pixelToRobot(l2.p1);
  const std::optional<Vector2f> l2End = cameraMatrix_->pixelToRobot(l2.p2);
  if (!l1Start.has_value() || !l1End.has_value() || !l2Start.has_value() || !l2End.has_value())
  {
    return false;
  }

  const Vector2f vec1 = l1End.value() - l1Start.value();
  const Vector2f vec2 = l2End.value() - l2Start.value();
  if (vec1.x() == 0 && vec1.y() == 0)
  {
    return false;
  }

  float angle = std::acos(vec1.normalized().dot(vec2.normalized()));
  float angleInDeg = angle / TO_RAD;
  debug().update(mount_ + ".AngleInDeg", angleInDeg);
  debug().update(mount_ + ".AngleInRad", angle);
  float angleThresholdInRad = angleThreshold_() * TO_RAD;
  return (angle > (M_PI_2 - angleThresholdInRad) && angle < (M_PI_2 + angleThresholdInRad));
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
  std::size_t const halfSize = points.size() / 2;
  VecVector2i leftGroup(points.begin(), points.begin() + halfSize);
  VecVector2i rightGroup(points.begin() + halfSize, points.end());
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
      if (segment.field >= 0.5f && segment.startEdgeType != EdgeType::BORDER)
      {
        borderPoints_.push_back(segment.start);
        break;
      }
    }
  }
  debug().update(mount_ + ".numBorderPoints", borderPoints_.size());
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
    debug().update(mount_ + ".numPointsFirstLine", firstLinePoints.size());

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
          // Count the border points used for the first line which would be above the second line
          int borderPointsNotEnclosed{0};
          for (const auto& point : firstLinePoints)
          {
            if (second.getY(point.x()) > point.y())
            {
              borderPointsNotEnclosed++;
            }
          }
          debug().update(mount_ + ".borderPointsNotEnclosed", borderPointsNotEnclosed);
          if (borderPointsNotEnclosed < maxBorderPointsNotEnclosed_())
          {
            // Accept line
            fieldBorder_->borderLines.push_back(second);
            debug().update(mount_ + ".numPointsSecondLine", secondLinePoints.size());
            // Two have been found
          }
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
                                  unsigned int iterations, int maxDistance)
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
    if (imageData_->cameraPosition == CameraPosition::TOP)
    {
      // Find border points
      findBorderPoints();
      // Find the border lines
      findBorderLines();
    }
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
      fieldBorderImage.drawCircle(Image422::get444From422Vector(bp), 3, Color::BLACK);
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
      fieldBorderImage.drawLine(line444, Color::RED);
      fieldBorderImage.drawLine({line444.p1.x(), line444.p1.y() + 1},
                                {line444.p2.x(), line444.p2.y() + 1}, Color::RED);
      fieldBorderImage.drawLine({line444.p1.x(), line444.p1.y() + 1},
                                {line444.p2.x(), line444.p2.y() + 1}, Color::RED);
      fieldBorderImage.drawLine({line444.p1.x(), line444.p1.y() - 1},
                                {line444.p2.x(), line444.p2.y() - 1}, Color::RED);
    }
    debug().sendImage(mount_ + "." + imageData_->identification + "_image", fieldBorderImage);
  }
}
