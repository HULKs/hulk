#pragma once

#include <vector>

#include "Framework/Module.hpp"
#include "Tools/Math/Eigen.hpp"
#include "Tools/Storage/Image.hpp"

#include "Data/CameraMatrix.hpp"
#include "Data/FieldBorder.hpp"
#include "Data/GoalData.hpp"
#include "Data/ImageData.hpp"

struct VisionGoalPost
{
  Vector2i rising_edge;
  Vector2i falling_edge;
  Vector2i center;
  int height;
};

class Brain;

class GoalDetection : public Module<GoalDetection, Brain>
{
public:
  /**
   * GoalDetection constructor
   * @param manager a reference to the brain object
   */
  GoalDetection(const ModuleManagerInterface& manager);

  /**
   * Detects goal posts in current images
   */
  void cycle();

private:
  void matchBorderEdges(const Image& image);
  void checkGoalPosts(const Image& image);
  void sendImageForDebug(const Image& image);
  /// the detected rising edges in the Y channel on the field border
  VecVector2i rising_edges_;
  /// the detected falling edges in the Y channel on the field border
  VecVector2i falling_edges_;
  /// the detected goal posts in an internal representation
  std::vector<VisionGoalPost> goal_posts_;
  /// a reference to the current image
  const Dependency<ImageData> image_data_;
  /// a reference to the camera matrix
  const Dependency<CameraMatrix> camera_matrix_;
  /// a reference to the field border
  const Dependency<FieldBorder> field_border_;
  /// a reference to the result stored in the database
  Production<GoalData> goal_data_;
};
