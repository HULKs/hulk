#pragma once

#include "Data/BodyPose.hpp"
#include "Data/CameraMatrix.hpp"
#include "Data/FieldBorder.hpp"
#include "Data/FieldDimensions.hpp"
#include "Data/FilteredSegments.hpp"
#include "Data/ImageData.hpp"
#include "Data/ImageSegments.hpp"
#include "Data/RobotData.hpp"
#include "Framework/Module.hpp"
#include "Tools/Math/Rectangle.hpp"

struct Column
{
  Column()
    : seedPosition(Vector2i::Zero())
  {
  }
  explicit Column(Vector2i seed)
    : seedPosition(std::move(seed))
  {
  }
  /// position of the seed (last edgepoint in column)
  Vector2i seedPosition;
  /// buffer for the calculated y median position
  int seedPositionYMedian{0};
  /// y positions of the edge points in the column
  std::vector<int> edgePointsY;
  /// flag to keep track that this column was visited - i.e. that the seed was evaluated
  bool visited{false};
  /// flag to keep track that this column is deleted - i.e. all seeds and edgepoints are discarded
  bool deleted{false};
  /// convenience function to get the column's x position in pixel coordinates
  int x() const
  {
    return seedPosition.x();
  }
};

struct Candidate
{
  Candidate() = default;
  /// candidate box in pixel coordinates
  Rectangle<int> box{Vector2i::Zero(), Vector2i::Zero()};
  /// the number of edge points present in the box
  unsigned int numberEdgePoints{0};
};


class Brain;

/**
 * @brief The RobotDetection class
 */
class RobotDetection : public Module<RobotDetection, Brain>
{
public:
  /// the name of this module
  ModuleName name__{"RobotDetection"};
  /**
   * @brief the constructor of this module
   * @param manager the module manager interface
   */
  explicit RobotDetection(const ModuleManagerInterface& manager);
  /**
   * @brief cycle writes the position of other robots to the production
   */
  void cycle() override;

private:
  /// the bodypose
  const Dependency<BodyPose> bodyPose_;
  /// the cameraMatrix
  const Dependency<CameraMatrix> cameraMatrix_;
  /// the fieldBorder
  const Dependency<FieldBorder> fieldBorder_;
  /// the field dimensions
  const Dependency<FieldDimensions> fieldDimensions_;
  /// the currently processed image
  const Dependency<ImageData> imageData_;
  /// the result of the image segmentation
  const Dependency<ImageSegments> imageSegments_;

  /// the height of the detection box in m
  const Parameter<float> detectionBoxHeight_;
  /// the width of the detection box in m
  const Parameter<float> detectionBoxWidth_;
  /// threshold for minimum number of consecutive non-field segments below the field border to be
  /// considered as edgepoints
  const Parameter<int> minConsecutiveSegments_;
  /// threshold for minimum number of edgepoints in a candidate box for an accepted candidate
  const Parameter<unsigned int> minEdgePointsInCandidateBox_;
  /// draw edgepoints in the debug image
  const Parameter<bool> drawEdgePoints_;
  /// draw seeds (last edgepoint in a column) in the debug image
  const Parameter<bool> drawSeeds_;
  /// draw boxes for accepted candidates
  const Parameter<bool> drawAcceptedCandidates_;
  /// draw boxes for cut candidates (candidates that would be accepted but are located at the
  /// image's bottom)
  const Parameter<bool> drawCutCandidates_;
  /// draw rejected candidates (not enough edgepoints in the candidate box)
  const Parameter<bool> drawRejectedCandidates_;
  /// draw evaluation windows for the seeds i.e. the bounding box of the possible candidate boxes
  /// for a specific seed
  const Parameter<bool> drawWindows_;

  /// positions of other robots in robot coordinates
  Production<RobotData> robotData_;

  /// vector of columns - there is one column for every scanline
  std::vector<Column> columns_;
  /// box position and number of edgepoints of accepted candidates
  std::vector<std::pair<Rectangle<int>, int>> debugAcceptedBoxes_;
  /// box position and number of edgepoints of cut candidates
  std::vector<std::pair<Rectangle<int>, int>> debugCutBoxes_;
  /// box position and number of edgepoints of rejected candidates
  std::vector<std::pair<Rectangle<int>, int>> debugRejectedBoxes_;
  /// position of evaluation windows
  std::vector<Rectangle<int>> debugWindows_;
  /**
   * @brief setup a column for every scanline in the image that holds information about the
   * edgepoints, seeds and status of the scanline
   */
  void setupColumns();
  /**
   * @brief iterates over all columns and changes the y position of the seeds to the median of its
   * seed and the two neighboring seeds
   */
  void medianSeeds();
  /**
   * @brief returns the columns with the seed with the highest y position that is not visited nor
   * deleted. This represents the seed that is closest to the robot.
   * @return column with the nearest valid seed
   */
  Column* getColumnWithNearestSeed();
  /**
   * @brief find best candidate for a given seed
   * @param seed postion of the seed to find the best candidate for
   * @param candidate reference to the candidate
   * @return bool that indicates wether finding the best candidate succeeded
   */
  bool findBestCandidate(const Vector2i& seed, Candidate& candidate);
  /**
   * @brief set all columns that pass through the given candidate box or the padding around the
   * candidate box to deleted
   * @param candidate the candidate for which to delete the columns
   * @param deletePaddingFactor describes how many additional columns at each side of the candidate
   * will be deleted in relation to the box size
   */
  void deleteColumns(const Candidate& candidate, float deletePaddingFactor);
  /**
   * @brief iterate over the nearest seeds, evaluate candidates and if accepted push back the robot
   * position
   */
  void findRobots();
  /**
   * @brief send debug image that shows the positions of all detected robots plus additional
   * information
   */
  void sendRobotPositionImageForDebug();
  /**
   * @brief send debug image that shows a histogram of the seeds over the x axis
   */
  void sendHistogramImageForDebug();
};
