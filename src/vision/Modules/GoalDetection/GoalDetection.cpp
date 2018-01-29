#include <algorithm>
#include <iterator>
#include <vector>

#include "Tools/Chronometer.hpp"
#include "Tools/Storage/Image.hpp"

#include "GoalDetection.hpp"
#include "print.hpp"

// TODO: Do real edge detection. 7 is a magic number.
#define RISING_EDGE(y) ((y) > 7)
#define FALLING_EDGE(y) ((y) < -7)

GoalDetection::GoalDetection(const ModuleManagerInterface& manager)
  : Module(manager, "GoalDetection")
  , image_data_(*this)
  , camera_matrix_(*this)
  , field_border_(*this)
  , goal_data_(*this)
{
}

void GoalDetection::matchBorderEdges(const Image& image)
{
  Vector2i point, peak;
  bool found_post;
  VisionGoalPost gp;
  VecVector2i border_points(field_border_->getBorderPoints(2));
  if (border_points.size() < 2)
  {
    return;
  }
  unsigned char y, y_last = image[border_points[0]].y_;
  int g, g_min = 8, g_max = -8;
  for (auto it = std::next(border_points.begin()); it != border_points.end(); it++)
  {
    point = *it;
    // It is better to search a bit below the field border because the field border itself has lots of edges.
    point.y() += 10;
    if (point.y() >= image.size_.y())
    {
      point.y() = image.size_.y() - 1;
    }
    // This is the same edge detection as in the former RegionClassifier.
    y = image[point].y_;
    g = y - y_last;
    if (g < g_min)
    {
      if (g_max > 8)
      {
        // Save rising edge to use it for matching with falling edges later.
        rising_edges_.push_back(peak);
      }
      g_min = g;
      g_max = -8;
      peak = point;
    }
    if (g > g_max)
    {
      if (g_min < -8)
      {
        // Look for a matching rising edge.
        for (auto rising_it = rising_edges_.begin(); rising_it != rising_edges_.end(); rising_it++)
        {
          // Check the distance between these points.
          // TODO: Do this with the projected distance and more sensible values.
          if ((peak.x() - rising_it->x()) > 45)
          {
            continue;
          }
          if ((peak.x() - rising_it->x()) < 15)
          {
            break;
          }
          // Check whether there is already a goal post between these points.
          found_post = false;
          for (auto post_it = goal_posts_.begin(); post_it != goal_posts_.end(); post_it++)
          {
            if (post_it->center.x() >= rising_it->x())
            {
              found_post = true;
              break;
            }
          }
          if (found_post)
          {
            continue;
          }
          // Now we have a goal post candidate.
          gp.rising_edge = *rising_it;
          gp.falling_edge = peak;
          gp.center = (gp.rising_edge + gp.falling_edge) / 2;
          gp.height = 0;
          goal_posts_.push_back(gp);
        }
        // Save falling edge - This is for the debug image only.
        falling_edges_.push_back(peak);
      }
      g_max = g;
      g_min = 8;
      peak = point;
    }
    // Update last y value.
    y_last = y;
  }
}

void GoalDetection::checkGoalPosts(const Image& image)
{
  Vector2i point;
  Vector2f projected_point;
  int y_last, y_diff;
  bool found_rising, found_falling;
  // Now there is a vector with goal post candidates.
  // In the following loop they are checked for further criteria and erased.
  for (auto it = goal_posts_.begin(); it != goal_posts_.end();)
  {
    // This extends the goal post candidate up to the point where it ends.
    // The greater than comparison (as opposed to a greater/equal comparison) is needed because
    // when the point's y coordinate is zero the loop must not be executed anymore.
    for (point = it->center, y_last = image[point].y_; point.y() > 0; point.y()--, it->height++)
    {
      y_diff = image[point].y_ - y_last;
      if (FALLING_EDGE(y_diff))
      {
        break;
      }
      y_last += y_diff;
    }
    // This extends the goal post candidate down to the point where it ends.
    // TODO: This could use the field color
    // The reason for the - 1 is the same as for the > comparison above.
    for (y_last = image[it->center].y_; it->center.y() < (image.size_.y() - 1); it->center.y()++, it->height++)
    {
      y_diff = image[it->center].y_ - y_last;
      if (FALLING_EDGE(y_diff))
      {
        break;
      }
      y_last += y_diff;
    }
    // If the goal post candidate is lower than 30 pixels it is discarded.
    if (it->height < 60)
    {
      it = goal_posts_.erase(it);
      continue;
    }
    // Find corresponding edges above the field border.
    found_rising = found_falling = false;
    for (point.x() = std::max(0, it->rising_edge.x() - 7), point.y() = std::max(0, it->center.y() - 8), y_last = image[point].y_;
         point.x() < std::min(image.size_.x(), it->falling_edge.x() + 8); point.x()++)
    {
      y_diff = image[point].y_ - y_last;
      if (RISING_EDGE(y_diff) && (abs(point.x() - it->rising_edge.x()) < 8))
      {
        // This rising edge probably corresponds with the previously found one.
        found_rising = true;
      }
      else if (FALLING_EDGE(y_diff) && (abs(point.x() - it->falling_edge.x()) < 8))
      {
        // This falling edge probably corresponds with the previously found one.
        found_falling = true;
      }
      y_last += y_diff;
    }
    // If there are no corresponding edges the goal post candidate is discarded.
    if (!(found_rising && found_falling))
    {
      it = goal_posts_.erase(it);
      continue;
    }
    // The center point is projected into the robot coordinate system and stored for later usage.
    camera_matrix_->pixelToRobot(it->center, projected_point);
    goal_data_->posts.push_back(projected_point);
    it++;
  }
}

void GoalDetection::sendImageForDebug(const Image& image)
{
  if (!debug().isSubscribed(mount_ + "." + image_data_->identification + "_image"))
  {
    return;
  }

  Image goalDetectionImage(image);
  for (auto it = rising_edges_.begin(); it != rising_edges_.end(); it++)
  {
    goalDetectionImage[*it] = Color::BLACK;
  }
  for (auto it = falling_edges_.begin(); it != falling_edges_.end(); it++)
  {
    goalDetectionImage[*it] = Color::WHITE;
  }
  for (auto it = goal_posts_.begin(); it != goal_posts_.end(); it++)
  {
    goalDetectionImage.circle(it->center, 5, Color::RED);
    goalDetectionImage.line(it->center, Vector2i(it->center.x(), it->center.y() - it->height), Color::GREEN);
  }
  debug().sendImage(mount_ + "." + image_data_->identification + "_image", goalDetectionImage);
}

void GoalDetection::cycle()
{
  const Image& image = image_data_->image;
  {
    Chronometer time(debug(), mount_ + ".cycle_time");

    // Clear member variables
    rising_edges_.clear();
    falling_edges_.clear();
    goal_posts_.clear();

    goal_data_->timestamp = image_data_->timestamp;
    matchBorderEdges(image);
    checkGoalPosts(image);
    goal_data_->valid = true;
  }
  sendImageForDebug(image);
}
