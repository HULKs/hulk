#include "FieldBorderDetection.hpp"
#include "Tools/Chronometer.hpp"
#include "Tools/Storage/Image.hpp"
#include "Utils/Algorithms.hpp"
#include "print.hpp"

#include "Definitions/windows_definition_fix.hpp"

FieldBorderDetection::FieldBorderDetection(const ModuleManagerInterface& manager)
  : Module(manager, "FieldBorderDetection")
  , angle_threshold_(*this, "angle_threshold", [] {})
  , image_data_(*this)
  , image_regions_(*this)
  , camera_matrix_(*this)
  , field_border_(*this)
  , filtered_regions_(*this)
{
}

bool FieldBorderDetection::isOrthogonal(const Line<int>& l1, const Line<int>& l2)
{
  Vector2f l1_start, l1_end, l2_start, l2_end;
  camera_matrix_->pixelToRobot(l1.p1, l1_start);
  camera_matrix_->pixelToRobot(l2.p1, l2_start);
  camera_matrix_->pixelToRobot(l1.p2, l1_end);
  camera_matrix_->pixelToRobot(l2.p2, l2_end);

  Vector2f vec1 = l1_end - l1_start;
  Vector2f vec2 = l2_end - l2_start;

  float angle = std::acos(vec1.normalized().dot(vec2.normalized()));
  float angle_in_deg = angle / TO_RAD;
  debug().update(mount_ + ".AngleInDeg", angle_in_deg);
  debug().update(mount_ + ".AngleInRad", angle);
  if (angle > (M_PI_2 - angle_threshold_() * TO_RAD) && angle < (M_PI_2 + angle_threshold_() * TO_RAD) && (vec1.x() != 0 || vec1.y() != 0))
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
  for (auto it = group.begin(); it != group.end(); it++)
  {
    center.x() += it->x();
    center.y() += it->y();
  }
  center.x() = (center.x() / group.size());
  center.y() = (center.y() / group.size());
  return center;
}

Line<int> FieldBorderDetection::bestFitLine(VecVector2i points)
{
  // Divide points into two equal groups (sorted from left to right)
  std::size_t const half_size = points.size() / 2;
  VecVector2i left_group(points.begin(), points.begin() + half_size);
  VecVector2i right_group(points.begin() + half_size, points.end());
  // Evaluate the center off both groups
  Vector2i left_center = centerOfGroup(left_group);
  Vector2i right_center = centerOfGroup(right_group);
  // Return line through both centers
  return Line<int>(left_center, right_center);
}

void FieldBorderDetection::findBorderPoints()
{
  for (auto& it : image_regions_->scanlines)
  {
    for (auto& it2 : it.regions)
    {
      if (it2.field > 0.5)
      {
        border_points_.emplace_back(it.x, it2.start);
        break;
      }
    }
  }
}

void FieldBorderDetection::findBorderLines()
{
  VecVector2i first_line_points, second_line_points, first_unused_points, second_unused_points;
  // Find points for the first line
  Line<int> line; // temporary dummy line
  if (!Algorithms::ransacLine(line, border_points_, first_line_points, first_unused_points, 20, 2))
  {
    return;
  }

  if (first_line_points.size() >= 5)
  {
    // Accept line
    Line<int> first;
    first = bestFitLine(first_line_points);
    field_border_->border_lines.push_back(first);

    // If enough points are left, check if second line exists
    if (first_unused_points.size() >= 5 && Algorithms::ransacLine(line, first_unused_points, second_line_points, second_unused_points, 20, 4))
    {
      if (second_line_points.size() >= 5)
      {
        Line<int> second;
        second = bestFitLine(second_line_points);
        // Check if second line is orthogonal to first
        if (isOrthogonal(first, second))
        {
          // Accept line
          field_border_->border_lines.push_back(second);
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

void FieldBorderDetection::createFilteredRegions()
{
  for (auto& it : image_regions_->scanlines)
  {
    Scanline scanline;
    scanline.id = it.id;
    scanline.x = it.x;
    bool below = false;
    for (auto& it2 : it.regions)
    {
      if (!below && field_border_->isInsideField(Vector2i(it.x, it2.start)))
      {
        below = true;
      }
      if (!below || it2.field > 0.5)
      {
        continue;
      }
      scanline.regions.push_back(it2);
    }
    filtered_regions_->scanlines.push_back(scanline);
  }
  filtered_regions_->valid = true;
}

void FieldBorderDetection::cycle()
{
  if (!image_regions_->valid)
  {
    return;
  }
  {
    Chronometer time(debug(), mount_ + ".cycle_time");
    // Reset private members
    border_points_.clear();
    field_border_->image_size = image_data_->image.size_;
    // Find border points
    findBorderPoints();
    // Find the border lines
    findBorderLines();
    createFilteredRegions();
  }
  sendImagesForDebug();
}

void FieldBorderDetection::sendImagesForDebug()
{
  if (!debug().isSubscribed(mount_ + "." + image_data_->identification + "_image"))
  {
    return;
  }

  Image fieldBorderImage(image_data_->image);

  for (auto it = border_points_.begin(); it != border_points_.end(); it++)
  {
    fieldBorderImage.circle(*it, 3, Color::BLACK);
  }

  VecVector2i all_border_points = field_border_->getBorderPoints();
  for (auto it = all_border_points.begin(); it != all_border_points.end(); it++)
  {
    fieldBorderImage[*it] = Color::BLUE;
  }

  for (auto it = field_border_->border_lines.begin(); it != field_border_->border_lines.end(); it++)
  {
    fieldBorderImage.line(it->p1, it->p2, Color::RED);
    fieldBorderImage.line(Vector2i(it->p1.x(), it->p1.y() + 1), Vector2i(it->p2.x(), it->p2.y() + 1), Color::RED);
    fieldBorderImage.line(Vector2i(it->p1.x(), it->p1.y() - 1), Vector2i(it->p2.x(), it->p2.y() - 1), Color::RED);
  }
  debug().sendImage(mount_ + "." + image_data_->identification + "_image", fieldBorderImage);
}
